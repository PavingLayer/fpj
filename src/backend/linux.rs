use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::MountBackend;
use crate::error::{LayerfsError, Result};

/// Linux backend using `fuse-overlayfs` for overlays and `bindfs` for bind mounts.
pub struct LinuxBackend;

impl Default for LinuxBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxBackend {
    pub fn new() -> Self {
        Self
    }

    fn run_fuse_tool(
        name: &str,
        args: &[&std::ffi::OsStr],
    ) -> std::result::Result<(), LayerfsError> {
        let output = Command::new(name).args(args).output().map_err(|e| {
            LayerfsError::Backend(format!(
                "{name} not available (install it with your package manager): {e}"
            ))
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LayerfsError::Backend(format!("{name} failed: {stderr}")));
        }

        Ok(())
    }
}

impl MountBackend for LinuxBackend {
    fn mount_overlay(
        &self,
        lower_dirs: &[PathBuf],
        upper_dir: &Path,
        work_dir: &Path,
        mount_point: &Path,
    ) -> Result<()> {
        fs::create_dir_all(upper_dir)?;
        fs::create_dir_all(work_dir)?;
        fs::create_dir_all(mount_point)?;

        let lowerdir = lower_dirs
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(":");

        let opts = format!(
            "allow_other,lowerdir={},upperdir={},workdir={}",
            lowerdir,
            upper_dir.display(),
            work_dir.display()
        );

        Self::run_fuse_tool(
            "fuse-overlayfs",
            &[
                std::ffi::OsStr::new("-o"),
                std::ffi::OsStr::new(&opts),
                mount_point.as_os_str(),
            ],
        )
    }

    fn unmount_overlay(&self, mount_point: &Path) -> Result<()> {
        Self::run_fuse_tool(
            "fusermount",
            &[std::ffi::OsStr::new("-u"), mount_point.as_os_str()],
        )
        .map_err(|e| LayerfsError::UnmountFailed {
            path: mount_point.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()> {
        self.ensure_writable_in_overlay(target)?;

        Self::run_fuse_tool("bindfs", &[source.as_os_str(), target.as_os_str()])
    }

    fn unbind_mount(&self, target: &Path) -> Result<()> {
        if !self.is_mounted(target)? {
            return Ok(());
        }

        Self::run_fuse_tool(
            "fusermount",
            &[std::ffi::OsStr::new("-u"), target.as_os_str()],
        )
        .map_err(|e| LayerfsError::UnmountFailed {
            path: target.to_path_buf(),
            reason: e.to_string(),
        })
    }

    fn is_mounted(&self, path: &Path) -> Result<bool> {
        let mounts = fs::read_to_string("/proc/mounts")?;
        let path_str = path.to_string_lossy();
        Ok(mounts.lines().any(|line| {
            line.split_whitespace()
                .nth(1)
                .is_some_and(|mp| mp == path_str.as_ref())
        }))
    }

    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        let marker = path.join(".fpj-copyup-marker");
        fs::write(&marker, b"")?;
        let _ = fs::remove_file(&marker);
        Ok(())
    }
}
