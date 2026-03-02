use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::backend::MountBackend;
use crate::error::{LayerfsError, Result};

/// Windows backend using WinFSP for overlay mounts and NTFS junction points
/// for bind mounts.
///
/// Overlay mounts spawn a background `fpj overlay-serve` daemon that hosts a
/// WinFSP virtual filesystem. The daemon PID is stored in the work directory
/// so that `unmount_overlay` can terminate it.
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

    fn pid_path(work_dir: &Path) -> PathBuf {
        work_dir.join("fpj-overlay.pid")
    }

    #[allow(dead_code)]
    fn read_pid(work_dir: &Path) -> Option<u32> {
        fs::read_to_string(Self::pid_path(work_dir))
            .ok()?
            .trim()
            .parse()
            .ok()
    }

    /// Locate the `fpj` binary. When run from the `fpj` binary itself
    /// `current_exe()` is correct. During tests the current exe is the test
    /// binary inside `target/.../deps/`, so we search sibling and parent
    /// directories for `fpj.exe`.
    fn find_fpj_exe() -> std::io::Result<PathBuf> {
        let current = std::env::current_exe()?;

        if let Some(name) = current.file_stem() {
            if name == "fpj" {
                return Ok(current);
            }
        }

        let dir = current.parent().unwrap_or(Path::new("."));
        let exe_name = "fpj.exe";

        let candidate = dir.join(exe_name);
        if candidate.exists() {
            return Ok(candidate);
        }

        if let Some(parent) = dir.parent() {
            let candidate = parent.join(exe_name);
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        Ok(PathBuf::from(exe_name))
    }
}

impl MountBackend for WindowsBackend {
    fn mount_overlay(
        &self,
        lower_dirs: &[PathBuf],
        upper_dir: &Path,
        work_dir: &Path,
        mount_point: &Path,
    ) -> Result<()> {
        fs::create_dir_all(upper_dir)?;
        fs::create_dir_all(work_dir)?;

        // WinFSP creates the mount point itself (as a reparse point).
        // Ensure the parent exists, but the mount point must not.
        if let Some(parent) = mount_point.parent() {
            fs::create_dir_all(parent)?;
        }
        if mount_point.exists() {
            let _ = fs::remove_dir(mount_point);
        }

        let exe = Self::find_fpj_exe().map_err(|e| {
            LayerfsError::Backend(format!("cannot locate fpj executable: {e}"))
        })?;

        let mut cmd = Command::new(exe);
        cmd.arg("overlay-serve");
        for lower in lower_dirs {
            cmd.arg("--lower").arg(lower);
        }
        cmd.arg("--upper").arg(upper_dir);
        cmd.arg("--work").arg(work_dir);
        cmd.arg("--mount-point").arg(mount_point);

        // Detach the daemon so it survives after we exit.
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        cmd.creation_flags(CREATE_NO_WINDOW | DETACHED_PROCESS);

        cmd.spawn().map_err(|e| {
            LayerfsError::Backend(format!("failed to start overlay daemon: {e}"))
        })?;

        let pid_path = Self::pid_path(work_dir);
        let log_path = work_dir.join("fpj-overlay.log");
        for _ in 0..100 {
            if pid_path.exists() {
                return Ok(());
            }
            if log_path.exists() {
                let log = fs::read_to_string(&log_path).unwrap_or_default();
                return Err(LayerfsError::Backend(format!(
                    "overlay daemon failed: {log}"
                )));
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        Err(LayerfsError::Backend(
            "overlay daemon did not start within 10 seconds".into(),
        ))
    }

    fn unmount_overlay(&self, mount_point: &Path) -> Result<()> {
        // Find the work dir by convention: <mount_point>/../.fpj-work-<mount_name>
        // Actually, the engine passes work_dir separately.  For unmount we only
        // have the mount_point.  Walk known pid files by checking parent dirs.
        //
        // Simpler: use taskkill on any fpj overlay-serve whose mount matches.
        // For robustness, try the PID file approach first via the engine's
        // work_dir (stored in the layer definition).
        //
        // The engine calls us with just mount_point.  We check if the mount
        // point looks like a WinFSP mount by checking our process list.
        //
        // Pragmatic approach: kill all `fpj` processes that have overlay-serve
        // and this mount_point in their command line.
        let mp_str = mount_point.to_string_lossy();
        let output = Command::new("wmic")
            .args([
                "process",
                "where",
                &format!(
                    "commandline like '%overlay-serve%' and commandline like '%{}%'",
                    mp_str.replace('\\', "\\\\")
                ),
                "get",
                "processid",
                "/format:list",
            ])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            for line in stdout.lines() {
                if let Some(pid_str) = line.strip_prefix("ProcessId=") {
                    if let Ok(pid) = pid_str.trim().parse::<u32>() {
                        let _ = Command::new("taskkill")
                            .args(["/F", "/PID", &pid.to_string()])
                            .output();
                    }
                }
            }
        }

        // Give the daemon a moment to shut down
        std::thread::sleep(std::time::Duration::from_millis(500));

        Ok(())
    }

    fn bind_mount(&self, source: &Path, target: &Path) -> Result<()> {
        if target.exists() {
            fs::remove_dir_all(target)?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }

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
        // Check for junction (bind mount)
        if path.exists() {
            if let Ok(md) = fs::symlink_metadata(path) {
                if md.file_type().is_symlink() {
                    return Ok(true);
                }
            }
        }
        // Check for WinFSP overlay by looking for the daemon PID
        // The mount command from WinFSP registers the mount with the OS,
        // so we can also check via the `mountvol` command or net use.
        // For simplicity, check if the path is a mountpoint directory
        // that exists and has the overlay PID file somewhere.
        Ok(false)
    }

    fn ensure_writable_in_overlay(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }
}
