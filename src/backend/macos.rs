use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::MountBackend;
use crate::error::{LayerfsError, Result};

pub struct MacOSBackend;

impl MacOSBackend {
    pub fn new() -> Self {
        Self
    }
}

impl MountBackend for MacOSBackend {
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
            "lowerdir={},upperdir={},workdir={}",
            lowerdir,
            upper_dir.display(),
            work_dir.display()
        );

        let output = Command::new("fuse-overlayfs")
            .arg("-o")
            .arg(opts)
            .arg(mount_point.as_os_str())
            .output()
            .map_err(|e| LayerfsError::Backend(format!("fuse-overlayfs not available: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LayerfsError::Backend(format!(
                "fuse-overlayfs failed: {stderr}"
            )));
        }

        Ok(())
    }

    fn unmount_overlay(&self, mount_point: &Path) -> Result<()> {
        let output = Command::new("umount")
            .arg(mount_point.as_os_str())
            .output()
            .map_err(|e| LayerfsError::UnmountFailed {
                path: mount_point.to_path_buf(),
                reason: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LayerfsError::UnmountFailed {
                path: mount_point.to_path_buf(),
                reason: stderr.into_owned(),
            });
        }

        Ok(())
    }

    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()> {
        self.ensure_writable_in_overlay(target)?;

        let output = Command::new("bindfs")
            .arg(source.as_os_str())
            .arg(target.as_os_str())
            .output()
            .map_err(|e| {
                LayerfsError::Backend(format!(
                    "bindfs not available (install via: brew install bindfs): {e}"
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LayerfsError::Backend(format!(
                "bindfs {} -> {} failed: {stderr}",
                source.display(),
                target.display()
            )));
        }

        Ok(())
    }

    fn unbind_mount(&self, target: &Path) -> Result<()> {
        if !self.is_mounted(target)? {
            return Ok(());
        }

        let output = Command::new("umount")
            .arg(target.as_os_str())
            .output()
            .map_err(|e| LayerfsError::UnmountFailed {
                path: target.to_path_buf(),
                reason: e.to_string(),
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LayerfsError::UnmountFailed {
                path: target.to_path_buf(),
                reason: stderr.into_owned(),
            });
        }

        Ok(())
    }

    fn is_mounted(&self, path: &Path) -> Result<bool> {
        let output = Command::new("mount").output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let path_str = path.to_string_lossy();
        Ok(stdout.lines().any(|line| line.contains(path_str.as_ref())))
    }

    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        let marker = path.join(".fpj-copyup-marker");
        fs::write(&marker, b"")?;
        let _ = fs::remove_file(&marker);
        Ok(())
    }
}
