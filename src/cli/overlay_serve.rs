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

    let log_path = work.join("fpj-overlay.log");

    let init = winfsp::winfsp_init();
    if let Err(e) = init {
        let _ = std::fs::write(&log_path, format!("winfsp_init failed: {e:?}\n"));
        return Err(fpj::error::LayerfsError::Backend(format!(
            "WinFSP init failed: {e:?}"
        )));
    }

    let context = OverlayFs::new(lower, upper.clone());
    let params = OverlayFs::volume_params();

    let mut host = FileSystemHost::new(params, context).map_err(|e| {
        let _ = std::fs::write(&log_path, format!("host creation failed: {e}\n"));
        fpj::error::LayerfsError::Backend(format!("WinFSP host creation failed: {e}"))
    })?;

    host.mount(&mount_point).map_err(|e| {
        let _ = std::fs::write(&log_path, format!("mount failed: {e}\n"));
        fpj::error::LayerfsError::Backend(format!(
            "WinFSP mount at {} failed: {e}",
            mount_point.display()
        ))
    })?;

    host.start().map_err(|e| {
        let _ = std::fs::write(&log_path, format!("dispatcher start failed: {e}\n"));
        fpj::error::LayerfsError::Backend(format!("WinFSP dispatcher start failed: {e}"))
    })?;

    let pid = std::process::id();
    let pid_file = work.join("fpj-overlay.pid");
    let _ = std::fs::write(&pid_file, pid.to_string());

    loop {
        std::thread::park();
    }
}
