//! WinFSP-based overlay filesystem for Windows.
//!
//! Presents a merged view of multiple read-only lower directories plus one
//! read-write upper directory, with copy-on-write semantics and whiteout
//! support (`.wh.<name>` marker files in upper).
#![cfg(target_os = "windows")]

use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Seek, SeekFrom, Write as _};
use std::os::windows::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use widestring::U16CStr;
use winfsp::filesystem::*;
use winfsp::host::VolumeParams;
use winfsp::FspError;

const WHITEOUT_PREFIX: &str = ".wh.";
const FILE_DIRECTORY_FILE: u32 = 0x0000_0001;
const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;

const STATUS_OBJECT_NAME_NOT_FOUND: i32 = 0xC000_0034_u32 as i32;
const STATUS_OBJECT_NAME_COLLISION: i32 = 0xC000_0035_u32 as i32;
const STATUS_END_OF_FILE: i32 = 0xC000_0011_u32 as i32;

fn nt(code: i32) -> FspError {
    FspError::NTSTATUS(code)
}

// ---------------------------------------------------------------------------
// File context
// ---------------------------------------------------------------------------

enum Handle {
    File(fs::File),
    Dir(DirBuffer),
}

pub struct OverlayFileContext {
    state: Mutex<(PathBuf, Handle)>,
    rel: PathBuf,
    is_dir: bool,
}

// ---------------------------------------------------------------------------
// Overlay filesystem
// ---------------------------------------------------------------------------

pub struct OverlayFs {
    lower_dirs: Vec<PathBuf>,
    upper_dir: PathBuf,
}

impl OverlayFs {
    pub fn new(lower_dirs: Vec<PathBuf>, upper_dir: PathBuf) -> Self {
        Self {
            lower_dirs,
            upper_dir,
        }
    }

    pub fn volume_params() -> VolumeParams {
        let mut vp = VolumeParams::new();
        vp.filesystem_name("fpj-overlay")
            .sector_size(512)
            .sectors_per_allocation_unit(1)
            .max_component_length(255)
            .case_sensitive_search(false)
            .case_preserved_names(true)
            .unicode_on_disk(true)
            .persistent_acls(false)
            .post_cleanup_when_modified_only(true)
            .file_info_timeout(1000);
        vp
    }

    // -- path helpers -------------------------------------------------------

    fn to_rel(name: &U16CStr) -> PathBuf {
        let os: OsString = name.to_os_string();
        let p = Path::new(&os);
        p.strip_prefix("\\").unwrap_or(p).to_path_buf()
    }

    fn is_root(rel: &Path) -> bool {
        rel.as_os_str().is_empty()
    }

    fn resolve(&self, rel: &Path) -> Option<PathBuf> {
        if Self::is_root(rel) {
            return Some(self.upper_dir.clone());
        }

        let upper = self.upper_dir.join(rel);
        if upper.exists() {
            return Some(upper);
        }

        if self.has_whiteout(rel) {
            return None;
        }

        for lower in &self.lower_dirs {
            let p = lower.join(rel);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }

    fn in_upper(&self, rel: &Path) -> bool {
        Self::is_root(rel) || self.upper_dir.join(rel).exists()
    }

    // -- whiteout helpers ---------------------------------------------------

    fn whiteout_path(&self, rel: &Path) -> Option<PathBuf> {
        let name = rel.file_name()?;
        let wh = format!("{WHITEOUT_PREFIX}{}", name.to_string_lossy());
        let parent = rel.parent().unwrap_or(Path::new(""));
        Some(if parent.as_os_str().is_empty() {
            self.upper_dir.join(wh)
        } else {
            self.upper_dir.join(parent).join(wh)
        })
    }

    fn has_whiteout(&self, rel: &Path) -> bool {
        self.whiteout_path(rel)
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    fn create_whiteout(&self, rel: &Path) {
        if let Some(wh) = self.whiteout_path(rel) {
            if let Some(p) = wh.parent() {
                let _ = fs::create_dir_all(p);
            }
            let _ = fs::write(&wh, b"");
        }
    }

    fn remove_whiteout(&self, rel: &Path) {
        if let Some(wh) = self.whiteout_path(rel) {
            let _ = fs::remove_file(wh);
        }
    }

    // -- copy-on-write ------------------------------------------------------

    fn copy_up(&self, rel: &Path) -> std::io::Result<PathBuf> {
        let upper = self.upper_dir.join(rel);
        if upper.exists() {
            return Ok(upper);
        }
        if let Some(parent) = rel.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(self.upper_dir.join(parent))?;
        }
        for lower in &self.lower_dirs {
            let src = lower.join(rel);
            if src.exists() {
                if src.is_dir() {
                    fs::create_dir_all(&upper)?;
                } else {
                    fs::copy(&src, &upper)?;
                }
                return Ok(upper);
            }
        }
        Ok(upper)
    }

    fn ensure_upper_file(
        &self,
        ctx: &OverlayFileContext,
    ) -> winfsp::Result<()> {
        let mut guard = ctx.state.lock().unwrap();
        if self.in_upper(&ctx.rel) {
            return Ok(());
        }
        let upper = self.copy_up(&ctx.rel).map_err(|e| FspError::IO(e.kind()))?;
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&upper)
            .map_err(|e| FspError::IO(e.kind()))?;
        *guard = (upper, Handle::File(file));
        Ok(())
    }

    // -- directory merging --------------------------------------------------

    fn merged_entries(&self, rel: &Path) -> Vec<(OsString, PathBuf)> {
        let mut seen = BTreeMap::<OsString, PathBuf>::new();
        let mut whiteouts = HashSet::<OsString>::new();

        let upper = if Self::is_root(rel) {
            self.upper_dir.clone()
        } else {
            self.upper_dir.join(rel)
        };

        if let Ok(rd) = fs::read_dir(&upper) {
            for e in rd.flatten() {
                let n = e.file_name();
                let s = n.to_string_lossy();
                if let Some(hidden) = s.strip_prefix(WHITEOUT_PREFIX) {
                    whiteouts.insert(OsString::from(hidden));
                } else {
                    seen.insert(n, e.path());
                }
            }
        }

        for lower in &self.lower_dirs {
            let dir = if Self::is_root(rel) {
                lower.clone()
            } else {
                lower.join(rel)
            };
            if let Ok(rd) = fs::read_dir(&dir) {
                for e in rd.flatten() {
                    let n = e.file_name();
                    if !seen.contains_key(&n) && !whiteouts.contains(&n) {
                        seen.insert(n, e.path());
                    }
                }
            }
        }

        seen.into_iter().collect()
    }

    // -- metadata helpers ---------------------------------------------------

    fn fill_info(md: &fs::Metadata, fi: &mut FileInfo) {
        fi.file_attributes = md.file_attributes();
        fi.file_size = md.file_size();
        fi.allocation_size = (md.file_size() + 4095) & !4095;
        fi.creation_time = md.creation_time();
        fi.last_access_time = md.last_access_time();
        fi.last_write_time = md.last_write_time();
        fi.change_time = md.last_write_time();
    }

    fn refresh_info(path: &Path, fi: &mut FileInfo) -> winfsp::Result<()> {
        let md = fs::metadata(path).map_err(|e| FspError::IO(e.kind()))?;
        Self::fill_info(&md, fi);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// FileSystemContext implementation
// ---------------------------------------------------------------------------

impl FileSystemContext for OverlayFs {
    type FileContext = OverlayFileContext;

    fn get_security_by_name(
        &self,
        file_name: &U16CStr,
        _security_descriptor: Option<&mut [std::ffi::c_void]>,
        _reparse: impl FnOnce(&U16CStr) -> Option<FileSecurity>,
    ) -> winfsp::Result<FileSecurity> {
        let rel = Self::to_rel(file_name);
        let real = self.resolve(&rel).ok_or(nt(STATUS_OBJECT_NAME_NOT_FOUND))?;
        let md = fs::metadata(&real).map_err(|_| nt(STATUS_OBJECT_NAME_NOT_FOUND))?;
        Ok(FileSecurity {
            reparse: false,
            sz_security_descriptor: 0,
            attributes: md.file_attributes(),
        })
    }

    fn open(
        &self,
        file_name: &U16CStr,
        _create_options: u32,
        _granted_access: u32,
        file_info: &mut OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        let rel = Self::to_rel(file_name);
        let real = self.resolve(&rel).ok_or(nt(STATUS_OBJECT_NAME_NOT_FOUND))?;
        let md = fs::metadata(&real).map_err(|_| nt(STATUS_OBJECT_NAME_NOT_FOUND))?;
        Self::fill_info(&md, file_info.as_mut());

        let is_dir = md.is_dir();
        let handle = if is_dir {
            Handle::Dir(DirBuffer::new())
        } else {
            let f = fs::OpenOptions::new()
                .read(true)
                .open(&real)
                .map_err(|e| FspError::IO(e.kind()))?;
            Handle::File(f)
        };

        Ok(OverlayFileContext {
            state: Mutex::new((real, handle)),
            rel,
            is_dir,
        })
    }

    fn close(&self, _context: Self::FileContext) {}

    fn create(
        &self,
        file_name: &U16CStr,
        create_options: u32,
        _granted_access: u32,
        _file_attributes: u32,
        _security_descriptor: Option<&[std::ffi::c_void]>,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        _extra_buffer_is_reparse_point: bool,
        file_info: &mut OpenFileInfo,
    ) -> winfsp::Result<Self::FileContext> {
        let rel = Self::to_rel(file_name);
        if self.resolve(&rel).is_some() {
            return Err(nt(STATUS_OBJECT_NAME_COLLISION));
        }

        let upper = self.upper_dir.join(&rel);
        if let Some(p) = upper.parent() {
            fs::create_dir_all(p).map_err(|e| FspError::IO(e.kind()))?;
        }

        let is_dir = create_options & FILE_DIRECTORY_FILE != 0;
        if is_dir {
            fs::create_dir_all(&upper).map_err(|e| FspError::IO(e.kind()))?;
        } else {
            fs::File::create(&upper).map_err(|e| FspError::IO(e.kind()))?;
        }

        self.remove_whiteout(&rel);

        let md = fs::metadata(&upper).map_err(|e| FspError::IO(e.kind()))?;
        Self::fill_info(&md, file_info.as_mut());

        let handle = if is_dir {
            Handle::Dir(DirBuffer::new())
        } else {
            let f = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(&upper)
                .map_err(|e| FspError::IO(e.kind()))?;
            Handle::File(f)
        };

        Ok(OverlayFileContext {
            state: Mutex::new((upper, handle)),
            rel,
            is_dir,
        })
    }

    fn get_file_info(
        &self,
        context: &Self::FileContext,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        let guard = context.state.lock().unwrap();
        Self::refresh_info(&guard.0, file_info)
    }

    fn set_basic_info(
        &self,
        context: &Self::FileContext,
        _file_attributes: u32,
        _creation_time: u64,
        _last_access_time: u64,
        _last_write_time: u64,
        _last_change_time: u64,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        let guard = context.state.lock().unwrap();
        Self::refresh_info(&guard.0, file_info)
    }

    fn set_file_size(
        &self,
        context: &Self::FileContext,
        new_size: u64,
        set_allocation_size: bool,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        if !set_allocation_size {
            self.ensure_upper_file(context)?;
            let guard = context.state.lock().unwrap();
            if let Handle::File(ref f) = guard.1 {
                f.set_len(new_size).map_err(|e| FspError::IO(e.kind()))?;
            }
        }
        let guard = context.state.lock().unwrap();
        Self::refresh_info(&guard.0, file_info)
    }

    fn read(
        &self,
        context: &Self::FileContext,
        buffer: &mut [u8],
        offset: u64,
    ) -> winfsp::Result<u32> {
        let mut guard = context.state.lock().unwrap();
        match guard.1 {
            Handle::File(ref mut f) => {
                f.seek(SeekFrom::Start(offset))
                    .map_err(|e| FspError::IO(e.kind()))?;
                let n = f.read(buffer).map_err(|e| FspError::IO(e.kind()))?;
                if n == 0 {
                    Err(nt(STATUS_END_OF_FILE))
                } else {
                    Ok(n as u32)
                }
            }
            Handle::Dir(_) => Err(nt(STATUS_END_OF_FILE)),
        }
    }

    fn write(
        &self,
        context: &Self::FileContext,
        buffer: &[u8],
        offset: u64,
        write_to_eof: bool,
        _constrained_io: bool,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<u32> {
        self.ensure_upper_file(context)?;
        let mut guard = context.state.lock().unwrap();
        match guard.1 {
            Handle::File(ref mut f) => {
                if write_to_eof {
                    f.seek(SeekFrom::End(0))
                        .map_err(|e| FspError::IO(e.kind()))?;
                } else {
                    f.seek(SeekFrom::Start(offset))
                        .map_err(|e| FspError::IO(e.kind()))?;
                }
                let n = f.write(buffer).map_err(|e| FspError::IO(e.kind()))?;
                f.flush().map_err(|e| FspError::IO(e.kind()))?;
                Self::refresh_info(&guard.0, file_info)?;
                Ok(n as u32)
            }
            Handle::Dir(_) => Err(nt(STATUS_END_OF_FILE)),
        }
    }

    fn overwrite(
        &self,
        context: &Self::FileContext,
        _file_attributes: u32,
        _replace_file_attributes: bool,
        _allocation_size: u64,
        _extra_buffer: Option<&[u8]>,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        self.ensure_upper_file(context)?;
        let guard = context.state.lock().unwrap();
        if let Handle::File(ref f) = guard.1 {
            f.set_len(0).map_err(|e| FspError::IO(e.kind()))?;
        }
        Self::refresh_info(&guard.0, file_info)
    }

    fn flush(
        &self,
        context: Option<&Self::FileContext>,
        file_info: &mut FileInfo,
    ) -> winfsp::Result<()> {
        if let Some(ctx) = context {
            let mut guard = ctx.state.lock().unwrap();
            if let Handle::File(ref mut f) = guard.1 {
                f.flush().map_err(|e| FspError::IO(e.kind()))?;
            }
            Self::refresh_info(&guard.0, file_info)?;
        }
        Ok(())
    }

    fn cleanup(
        &self,
        context: &Self::FileContext,
        _file_name: Option<&U16CStr>,
        flags: u32,
    ) {
        const FspCleanupDelete: u32 = 0x01;
        if flags & FspCleanupDelete == 0 {
            return;
        }
        let guard = context.state.lock().unwrap();
        let real = &guard.0;
        let existed_in_lower = self
            .lower_dirs
            .iter()
            .any(|l| l.join(&context.rel).exists());

        if context.is_dir {
            let _ = fs::remove_dir_all(real);
        } else {
            let _ = fs::remove_file(real);
        }

        if existed_in_lower {
            self.create_whiteout(&context.rel);
        }
    }

    fn set_delete(
        &self,
        context: &Self::FileContext,
        _file_name: &U16CStr,
        delete_file: bool,
    ) -> winfsp::Result<()> {
        if !delete_file {
            return Ok(());
        }
        if context.is_dir {
            let entries = self.merged_entries(&context.rel);
            if !entries.is_empty() {
                return Err(FspError::NTSTATUS(0xC000_0101_u32 as i32));
            }
        }
        Ok(())
    }

    fn rename(
        &self,
        context: &Self::FileContext,
        _file_name: &U16CStr,
        new_file_name: &U16CStr,
        replace_if_exists: bool,
    ) -> winfsp::Result<()> {
        let new_rel = Self::to_rel(new_file_name);
        if !replace_if_exists && self.resolve(&new_rel).is_some() {
            return Err(nt(STATUS_OBJECT_NAME_COLLISION));
        }

        self.ensure_upper_file(context)?;
        let guard = context.state.lock().unwrap();
        let new_upper = self.upper_dir.join(&new_rel);
        if let Some(p) = new_upper.parent() {
            fs::create_dir_all(p).map_err(|e| FspError::IO(e.kind()))?;
        }
        fs::rename(&guard.0, &new_upper).map_err(|e| FspError::IO(e.kind()))?;

        let existed_in_lower = self
            .lower_dirs
            .iter()
            .any(|l| l.join(&context.rel).exists());
        if existed_in_lower {
            self.create_whiteout(&context.rel);
        }
        self.remove_whiteout(&new_rel);

        Ok(())
    }

    fn read_directory(
        &self,
        context: &Self::FileContext,
        _pattern: Option<&U16CStr>,
        marker: DirMarker<'_>,
        buffer: &mut [u8],
    ) -> winfsp::Result<u32> {
        let guard = context.state.lock().unwrap();
        let Handle::Dir(ref dir_buf) = guard.1 else {
            return Err(nt(STATUS_END_OF_FILE));
        };

        if let Ok(lock) = dir_buf.acquire(marker.is_none(), None) {
            // "." entry
            if let Ok(md) = fs::metadata(&guard.0) {
                let mut di: DirInfo<255> = DirInfo::new();
                Self::fill_info(&md, di.file_info_mut());
                let _ = di.set_name(".");
                let _ = lock.write(&mut di);
            }

            // ".." entry
            if let Some(parent) = guard.0.parent() {
                if let Ok(md) = fs::metadata(parent) {
                    let mut di: DirInfo<255> = DirInfo::new();
                    Self::fill_info(&md, di.file_info_mut());
                    let _ = di.set_name("..");
                    let _ = lock.write(&mut di);
                }
            }

            for (name, path) in self.merged_entries(&context.rel) {
                if let Ok(md) = fs::metadata(&path) {
                    let mut di: DirInfo<255> = DirInfo::new();
                    Self::fill_info(&md, di.file_info_mut());
                    let _ = di.set_name(&name);
                    let _ = lock.write(&mut di);
                }
            }
        }

        Ok(dir_buf.read(marker, buffer))
    }

    fn get_volume_info(&self, out: &mut VolumeInfo) -> winfsp::Result<()> {
        out.total_size = 1024 * 1024 * 1024;
        out.free_size = 512 * 1024 * 1024;
        Ok(())
    }
}
