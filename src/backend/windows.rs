use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::MountBackend;
use crate::error::{LayerfsError, Result};

/// Windows backend using NTFS junction points for bind mounts.
/// Overlay support is limited to a copy-based strategy.
pub struct WindowsBackend;

impl Default for WindowsBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl WindowsBackend {
    pub fn new() -> Self {
        Self
    }
}

impl MountBackend for WindowsBackend {
    fn mount_overlay(
        &self,
        lower_dirs: &[PathBuf],
        upper_dir: &Path,
        _work_dir: &Path,
        mount_point: &Path,
    ) -> Result<()> {
        // Copy-based overlay: copy lowest layer first, then overlay higher layers on top
        fs::create_dir_all(upper_dir)?;
        fs::create_dir_all(mount_point)?;

        for lower in lower_dirs.iter().rev() {
            copy_dir_recursive(lower, mount_point)?;
        }

        // Copy upper layer on top (highest priority)
        if upper_dir.exists() {
            copy_dir_recursive(upper_dir, mount_point)?;
        }

        Ok(())
    }

    fn unmount_overlay(&self, mount_point: &Path) -> Result<()> {
        if mount_point.exists() {
            fs::remove_dir_all(mount_point).map_err(|e| LayerfsError::UnmountFailed {
                path: mount_point.to_path_buf(),
                reason: e.to_string(),
            })?;
        }
        Ok(())
    }

    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()> {
        if target.exists() {
            fs::remove_dir_all(target)?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

        // Create NTFS junction point
        let output = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(target.as_os_str())
            .arg(source.as_os_str())
            .output()
            .map_err(|e| LayerfsError::Backend(format!("mklink /J failed: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(LayerfsError::Backend(format!(
                "junction {} -> {} failed: {stderr}",
                source.display(),
                target.display()
            )));
        }

        Ok(())
    }

    fn unbind_mount(&self, target: &Path) -> Result<()> {
        if !target.exists() {
            return Ok(());
        }

        // Junctions are removed via rmdir (not recursive delete)
        let output = Command::new("cmd")
            .args(["/C", "rmdir"])
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
        // For junction-based approach, check if path is a junction/symlink
        Ok(path.exists() && fs::symlink_metadata(path)?.file_type().is_symlink())
    }

    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
