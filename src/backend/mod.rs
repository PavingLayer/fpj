use std::path::{Path, PathBuf};

use crate::error::Result;

pub trait MountBackend {
    fn mount_overlay(
        &self,
        lower_dirs: &[PathBuf],
        upper_dir: &Path,
        work_dir: &Path,
        mount_point: &Path,
    ) -> Result<()>;

    fn unmount_overlay(&self, mount_point: &Path) -> Result<()>;

    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()>;

    fn unbind_mount(&self, target: &Path) -> Result<()>;

    fn is_mounted(&self, path: &Path) -> Result<bool>;

    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()>;
}

#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "windows")]
pub mod windows;

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
