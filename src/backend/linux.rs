use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use nix::mount::{mount, umount, MsFlags};

use crate::backend::MountBackend;
use crate::error::{LayerfsError, Result};

pub struct LinuxBackend;

impl LinuxBackend {
    pub fn new() -> Self {
        Self
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
            "lowerdir={},upperdir={},workdir={}",
            lowerdir,
            upper_dir.display(),
            work_dir.display()
        );

        // Try fuse-overlayfs first (unprivileged)
        let output = Command::new("fuse-overlayfs")
            .arg("-o")
            .arg(format!("allow_other,{opts}"))
            .arg(mount_point.as_os_str())
            .output();

        match output {
            Ok(o) if o.status.success() => return Ok(()),
            Ok(o) => {
                let stderr = String::from_utf8_lossy(&o.stderr);
                // Fall through to kernel overlay if fuse-overlayfs not available
                if stderr.contains("not found") || stderr.contains("No such file") {
                    // fuse-overlayfs not installed, try kernel overlay
                } else {
                    return Err(LayerfsError::Backend(format!(
                        "fuse-overlayfs failed: {stderr}"
                    )));
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // fuse-overlayfs binary not found, try kernel overlay
            }
            Err(e) => return Err(e.into()),
        }

        // Fallback: kernel overlayfs (needs CAP_SYS_ADMIN)
        mount(
            Some("overlay"),
            mount_point,
            Some("overlay"),
            MsFlags::empty(),
            Some(opts.as_str()),
        )
        .map_err(|e| LayerfsError::Backend(format!("kernel overlay mount failed: {e}")))?;

        Ok(())
    }

    fn unmount_overlay(&self, mount_point: &Path) -> Result<()> {
        // Try fusermount first (for FUSE mounts)
        let output = Command::new("fusermount")
            .arg("-u")
            .arg(mount_point.as_os_str())
            .output();

        match output {
            Ok(o) if o.status.success() => return Ok(()),
            _ => {}
        }

        // Fallback: regular umount
        umount(mount_point).map_err(|e| LayerfsError::UnmountFailed {
            path: mount_point.to_path_buf(),
            reason: e.to_string(),
        })?;

        Ok(())
    }

    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()> {
        self.ensure_writable_in_overlay(target)?;

        mount(
            Some(source),
            target,
            None::<&str>,
            MsFlags::MS_BIND,
            None::<&str>,
        )
        .map_err(|e| {
            LayerfsError::Backend(format!(
                "bind mount {} -> {} failed: {e}",
                source.display(),
                target.display()
            ))
        })?;

        Ok(())
    }

    fn unbind_mount(&self, target: &Path) -> Result<()> {
        if !self.is_mounted(target)? {
            return Ok(());
        }

        umount(target).map_err(|e| LayerfsError::UnmountFailed {
            path: target.to_path_buf(),
            reason: e.to_string(),
        })?;

        Ok(())
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
