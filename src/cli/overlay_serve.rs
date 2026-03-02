//! Hidden daemon subcommand that hosts a WinFSP overlay filesystem.
//! Spawned by `WindowsBackend::mount_overlay` and runs until killed.
#![cfg(target_os = "windows")]

use std::path::PathBuf;

use fpj::error::Result;

pub fn handle(
    lower: Vec<PathBuf>,
    upper: PathBuf,
    work: PathBuf,
    mount_point: PathBuf,
) -> Result<()> {
    use fpj::backend::winfsp_overlay::OverlayFs;
    use winfsp::host::FileSystemHost;

    winfsp::winfsp_init_or_die();

    let context = OverlayFs::new(lower, upper.clone());
    let params = OverlayFs::volume_params();

    let mut host = FileSystemHost::new(params, context).map_err(|e| {
        fpj::error::LayerfsError::Backend(format!("WinFSP host creation failed: {e}"))
    })?;

    host.mount(&mount_point).map_err(|e| {
        fpj::error::LayerfsError::Backend(format!(
            "WinFSP mount at {} failed: {e}",
            mount_point.display()
        ))
    })?;

    host.start().map_err(|e| {
        fpj::error::LayerfsError::Backend(format!("WinFSP dispatcher start failed: {e}"))
    })?;

    // Write PID file so the parent process can find and stop us.
    let pid = std::process::id();
    let pid_file = work.join("fpj-overlay.pid");
    let _ = std::fs::write(&pid_file, pid.to_string());

    // Block until the process is terminated.  When the parent calls
    // TerminateProcess (via unmount_overlay), the WinFSP driver handles
    // unmounting automatically.
    loop {
        std::thread::park();
    }
}
