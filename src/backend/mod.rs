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
pub mod winfsp_overlay;

/// Selects which mount implementation to use on Linux.
///
/// On macOS and Windows this is ignored; the platform default is always used.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BackendKind {
    /// Use kernel overlayfs + `mount --bind` when running as root,
    /// otherwise fall back to FUSE.
    #[default]
    Auto,
    /// Force FUSE-based backend (`fuse-overlayfs` + `bindfs`).
    /// Works without root.
    Fuse,
    /// Force kernel-based backend (`mount -t overlay` + `mount --bind`).
    /// Requires `CAP_SYS_ADMIN`.  Produces non-FUSE mounts compatible
    /// with CRIU checkpoint/restore.
    Kernel,
}

/// Create the default [`MountBackend`] for the current OS (equivalent to
/// [`create_backend_with`]`(`[`BackendKind::Auto`]`)`.
pub fn create_backend() -> Box<dyn MountBackend> {
    create_backend_with(BackendKind::Auto)
}

/// Create a [`MountBackend`] for the requested [`BackendKind`].
pub fn create_backend_with(kind: BackendKind) -> Box<dyn MountBackend> {
    #[cfg(target_os = "linux")]
    {
        match kind {
            BackendKind::Kernel => Box::new(linux::LinuxKernelBackend::new()),
            BackendKind::Fuse => Box::new(linux::LinuxBackend::new()),
            BackendKind::Auto => {
                if nix::unistd::geteuid().is_root() {
                    Box::new(linux::LinuxKernelBackend::new())
                } else {
                    Box::new(linux::LinuxBackend::new())
                }
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        let _ = kind;
        Box::new(macos::MacOSBackend::new())
    }

    #[cfg(target_os = "windows")]
    {
        let _ = kind;
        Box::new(windows::WindowsBackend::new())
    }
}
