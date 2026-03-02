use std::path::{Path, PathBuf};

use crate::error::Result;

/// Platform-specific mount operations.
///
/// Each supported OS provides its own implementation: Linux uses
/// `fuse-overlayfs`/`bindfs`, macOS uses macFUSE, and Windows uses
/// WinFSP for overlays with NTFS junction points for bind mounts.
pub trait MountBackend {
    /// Create an overlay filesystem merging `lower_dirs` with `upper_dir` at `mount_point`.
    fn mount_overlay(
        &self,
        lower_dirs: &[PathBuf],
        upper_dir: &Path,
        work_dir: &Path,
        mount_point: &Path,
    ) -> Result<()>;

    /// Tear down a previously mounted overlay at `mount_point`.
    fn unmount_overlay(&self, mount_point: &Path) -> Result<()>;

    /// Bind `source` onto `target` so they share the same directory view.
    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()>;

    /// Remove a bind mount previously created at `target`.
    fn unbind_mount(&self, target: &Path) -> Result<()>;

    /// Return `true` if `path` is currently an active mount point.
    fn is_mounted(&self, path: &Path) -> Result<bool>;

    /// Ensure `path` is writable inside an overlay (triggers copy-up if needed).
    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()>;
}

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "windows")]
pub(crate) mod winfsp_overlay;

/// Create the [`MountBackend`] appropriate for the current operating system.
pub fn create_backend() -> Box<dyn MountBackend> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxBackend::new())
    }

    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOSBackend::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsBackend::new())
    }
}
