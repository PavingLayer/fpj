use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::MountBackend;
use crate::error::{LayerfsError, Result};

// -- Shared helpers ---------------------------------------------------------

fn is_mount_point(path: &Path) -> Result<bool> {
    let mounts = fs::read_to_string("/proc/mounts")?;
    let path_str = path.to_string_lossy();
    Ok(mounts.lines().any(|line| {
        line.split_whitespace()
            .nth(1)
            .is_some_and(|mp| mp == path_str.as_ref())
    }))
}

fn trigger_copy_up(path: &Path) -> Result<()> {
    fs::create_dir_all(path)?;
    let marker = path.join(".fpj-copyup-marker");
    fs::write(&marker, b"")?;
    let _ = fs::remove_file(&marker);
    Ok(())
}

fn build_lowerdir(lower_dirs: &[PathBuf]) -> String {
    lower_dirs
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(":")
}

// -- FUSE backend (fuse-overlayfs + bindfs) ---------------------------------

/// Linux backend using `fuse-overlayfs` for overlays and `bindfs` for bind mounts.
///
/// Runs entirely in user space and does not require root privileges.
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

        let opts = format!(
            "allow_other,lowerdir={},upperdir={},workdir={}",
            build_lowerdir(lower_dirs),
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

        Self::run_fuse_tool(
            "bindfs",
            &[
                std::ffi::OsStr::new("-o"),
                std::ffi::OsStr::new("nonempty"),
                source.as_os_str(),
                target.as_os_str(),
            ],
        )
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
        is_mount_point(path)
    }

    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()> {
        trigger_copy_up(path)
    }
}

// -- Kernel backend (mount syscall via nix) ---------------------------------

/// Linux backend using the kernel's native overlayfs and bind mount syscalls.
///
/// Produces real kernel mounts (not FUSE) that are compatible with
/// CRIU checkpoint/restore.  Requires `CAP_SYS_ADMIN` (typically root).
///
/// When running under `sudo`, directories created by this backend are
/// `chown`-ed to `SUDO_UID:SUDO_GID` so that the real user retains
/// write access to the overlay's upper layer and mount point.
pub struct LinuxKernelBackend;

impl Default for LinuxKernelBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl LinuxKernelBackend {
    pub fn new() -> Self {
        Self
    }

    fn chown_to_real_user(path: &Path) {
        if let (Some(uid_s), Some(gid_s)) =
            (std::env::var_os("SUDO_UID"), std::env::var_os("SUDO_GID"))
        {
            if let (Ok(uid), Ok(gid)) = (
                uid_s.to_string_lossy().parse::<u32>(),
                gid_s.to_string_lossy().parse::<u32>(),
            ) {
                let _ = nix::unistd::chown(
                    path,
                    Some(nix::unistd::Uid::from_raw(uid)),
                    Some(nix::unistd::Gid::from_raw(gid)),
                );
            }
        }
    }
}

impl MountBackend for LinuxKernelBackend {
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
        Self::chown_to_real_user(upper_dir);
        Self::chown_to_real_user(work_dir);
        Self::chown_to_real_user(mount_point);

        let opts = format!(
            "lowerdir={},upperdir={},workdir={}",
            build_lowerdir(lower_dirs),
            upper_dir.display(),
            work_dir.display()
        );

        nix::mount::mount(
            Some("overlay"),
            mount_point,
            Some("overlay"),
            nix::mount::MsFlags::empty(),
            Some(opts.as_str()),
        )
        .map_err(|e| {
            LayerfsError::Backend(format!(
                "kernel overlay mount at {} failed: {e} (requires root)",
                mount_point.display()
            ))
        })
    }

    fn unmount_overlay(&self, mount_point: &Path) -> Result<()> {
        if !self.is_mounted(mount_point)? {
            return Ok(());
        }
        nix::mount::umount(mount_point).map_err(|e| LayerfsError::UnmountFailed {
            path: mount_point.to_path_buf(),
            reason: format!("{e}"),
        })
    }

    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()> {
        self.ensure_writable_in_overlay(target)?;

        nix::mount::mount(
            Some(source),
            target,
            None::<&str>,
            nix::mount::MsFlags::MS_BIND,
            None::<&str>,
        )
        .map_err(|e| {
            LayerfsError::Backend(format!(
                "kernel bind mount {} -> {} failed: {e} (requires root)",
                source.display(),
                target.display()
            ))
        })
    }

    fn unbind_mount(&self, target: &Path) -> Result<()> {
        if !self.is_mounted(target)? {
            return Ok(());
        }

        nix::mount::umount(target).map_err(|e| LayerfsError::UnmountFailed {
            path: target.to_path_buf(),
            reason: format!("{e}"),
        })
    }

    fn is_mounted(&self, path: &Path) -> Result<bool> {
        is_mount_point(path)
    }

    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()> {
        trigger_copy_up(path)
    }
}
