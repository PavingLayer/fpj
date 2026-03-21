use std::fmt;
use std::path::Path;
use std::process::Command;

use fpj::error::Result;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Ok,
    Fail,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Ok => write!(f, " ok "),
            Status::Fail => write!(f, "FAIL"),
        }
    }
}

struct Check {
    status: Status,
    label: String,
    detail: Option<String>,
    fix: Option<String>,
}

impl Check {
    fn ok(label: impl Into<String>) -> Self {
        Self {
            status: Status::Ok,
            label: label.into(),
            detail: None,
            fix: None,
        }
    }

    fn ok_with(label: impl Into<String>, detail: impl Into<String>) -> Self {
        Self {
            status: Status::Ok,
            label: label.into(),
            detail: Some(detail.into()),
            fix: None,
        }
    }

    fn fail(label: impl Into<String>, fix: impl Into<String>) -> Self {
        Self {
            status: Status::Fail,
            label: label.into(),
            detail: None,
            fix: Some(fix.into()),
        }
    }

    fn fail_with(
        label: impl Into<String>,
        detail: impl Into<String>,
        fix: impl Into<String>,
    ) -> Self {
        Self {
            status: Status::Fail,
            label: label.into(),
            detail: Some(detail.into()),
            fix: Some(fix.into()),
        }
    }
}

struct Section {
    title: String,
    checks: Vec<Check>,
}

fn print_report(sections: &[Section]) {
    println!("fpj doctor -- system diagnostics\n");

    let mut failures = 0;

    for section in sections {
        println!("{}:", section.title);
        for check in &section.checks {
            let detail = check
                .detail
                .as_deref()
                .map(|d| format!(" ({d})"))
                .unwrap_or_default();
            println!("  [{}] {}{}", check.status, check.label, detail);
            if let Some(fix) = &check.fix {
                println!("         Fix: {fix}");
            }
            if check.status == Status::Fail {
                failures += 1;
            }
        }
        println!();
    }

    if failures == 0 {
        println!("All checks passed.");
    } else {
        println!(
            "{failures} issue{} found. Install the missing dependencies listed above.",
            if failures == 1 { "" } else { "s" }
        );
    }
}

fn print_json(sections: &[Section]) {
    let mut entries = Vec::new();
    for section in sections {
        for check in &section.checks {
            let status = match check.status {
                Status::Ok => "ok",
                Status::Fail => "fail",
            };
            let mut obj = format!(
                r#"{{"section":"{}","check":"{}","status":"{}""#,
                section.title, check.label, status
            );
            if let Some(d) = &check.detail {
                obj.push_str(&format!(r#","detail":"{}""#, d.replace('"', "\\\"")));
            }
            if let Some(f) = &check.fix {
                obj.push_str(&format!(r#","fix":"{}""#, f.replace('"', "\\\"")));
            }
            obj.push('}');
            entries.push(obj);
        }
    }
    println!("[{}]", entries.join(","));
}

fn tool_version(name: &str, flag: &str) -> Option<String> {
    Command::new(name)
        .arg(flag)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            let line = s.lines().next().unwrap_or("").trim().to_string();
            if line.is_empty() {
                let se = String::from_utf8_lossy(&o.stderr);
                se.lines().next().unwrap_or("").trim().to_string()
            } else {
                line
            }
        })
}

// ---------------------------------------------------------------------------
// Linux checks
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn check_overlay_deps() -> Vec<Check> {
    let mut checks = Vec::new();

    // /dev/fuse
    if Path::new("/dev/fuse").exists() {
        checks.push(Check::ok("/dev/fuse available"));
    } else {
        checks.push(Check::fail(
            "/dev/fuse not found",
            "sudo modprobe fuse",
        ));
    }

    // fuse-overlayfs
    match tool_version("fuse-overlayfs", "--version") {
        Some(v) => checks.push(Check::ok_with("fuse-overlayfs", v)),
        None => checks.push(Check::fail(
            "fuse-overlayfs not found",
            "sudo apt-get install fuse-overlayfs  (or equivalent for your distro)",
        )),
    }

    // fusermount
    match tool_version("fusermount", "-V") {
        Some(v) => checks.push(Check::ok_with("fusermount", v)),
        None => checks.push(Check::fail(
            "fusermount not found",
            "sudo apt-get install fuse3  (or equivalent for your distro)",
        )),
    }

    checks
}

#[cfg(target_os = "linux")]
fn check_bind_deps() -> Vec<Check> {
    match tool_version("bindfs", "--version") {
        Some(v) => vec![Check::ok_with("bindfs", v)],
        None => vec![Check::fail(
            "bindfs not found",
            "sudo apt-get install bindfs  (or equivalent for your distro)",
        )],
    }
}

// ---------------------------------------------------------------------------
// macOS checks
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn check_overlay_deps() -> Vec<Check> {
    let mut checks = Vec::new();

    // macFUSE kext
    let kext_loaded = Command::new("kextstat")
        .output()
        .map(|o| {
            let s = String::from_utf8_lossy(&o.stdout);
            s.contains("macfuse") || s.contains("osxfuse")
        })
        .unwrap_or(false);

    if kext_loaded {
        checks.push(Check::ok("macFUSE kernel extension loaded"));
    } else {
        checks.push(Check::fail_with(
            "macFUSE kernel extension not loaded",
            "macFUSE may be installed but needs a reboot, or is not installed",
            "brew install --cask macfuse  (then reboot)",
        ));
    }

    match tool_version("fuse-overlayfs", "--version") {
        Some(v) => checks.push(Check::ok_with("fuse-overlayfs", v)),
        None => checks.push(Check::fail(
            "fuse-overlayfs not found",
            "brew install fuse-overlayfs",
        )),
    }

    checks
}

#[cfg(target_os = "macos")]
fn check_bind_deps() -> Vec<Check> {
    match tool_version("bindfs", "--version") {
        Some(v) => vec![Check::ok_with("bindfs", v)],
        None => vec![Check::fail(
            "bindfs not found",
            "brew install gromgit/fuse/bindfs-mac",
        )],
    }
}

// ---------------------------------------------------------------------------
// Windows checks
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn check_overlay_deps() -> Vec<Check> {
    // Check WinFSP by looking for the install dir in the registry (via reg query)
    // or just try to find the DLL.
    let winfsp_found = std::env::var("PROGRAMFILES(X86)")
        .or_else(|_| std::env::var("PROGRAMFILES"))
        .map(|pf| Path::new(&pf).join("WinFsp").join("bin").exists())
        .unwrap_or(false);

    if winfsp_found {
        vec![Check::ok("WinFSP installed")]
    } else {
        vec![Check::fail(
            "WinFSP not found",
            "Download from https://winfsp.dev/ or run: choco install winfsp",
        )]
    }
}

#[cfg(target_os = "windows")]
fn check_bind_deps() -> Vec<Check> {
    // Junctions are built into NTFS, always available
    vec![Check::ok("NTFS junction points (built-in)")]
}

// ---------------------------------------------------------------------------
// Linux kernel overlay checks
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn check_kernel_overlay_deps() -> Vec<Check> {
    let mut checks = Vec::new();

    if Path::new("/sys/module/overlay").exists() {
        checks.push(Check::ok("overlay kernel module loaded"));
    } else {
        checks.push(Check::fail(
            "overlay kernel module not loaded",
            "sudo modprobe overlay",
        ));
    }

    if nix::unistd::geteuid().is_root() {
        checks.push(Check::ok("running as root (CAP_SYS_ADMIN)"));
    } else {
        checks.push(Check::fail_with(
            "not running as root",
            "kernel overlayfs requires CAP_SYS_ADMIN",
            "run with sudo: sudo fpj --backend kernel ...",
        ));
    }

    checks
}

#[cfg(not(target_os = "linux"))]
fn check_kernel_overlay_deps() -> Vec<Check> {
    vec![Check::fail_with(
        "kernel overlayfs",
        "only available on Linux",
        "",
    )]
}

// ---------------------------------------------------------------------------
// Smoke tests
// ---------------------------------------------------------------------------

fn run_smoke_test(backend: &dyn fpj::backend::MountBackend, label: &str) -> Check {
    let tmp = match tempfile::TempDir::new() {
        Ok(t) => t,
        Err(e) => return Check::fail_with(label, format!("temp dir: {e}"), ""),
    };

    let lower = tmp.path().join("lower");
    let upper = tmp.path().join("upper");
    let work = tmp.path().join("work");
    let merged = tmp.path().join("merged");

    for d in [&lower, &upper, &work, &merged] {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(lower.join("probe.txt"), "fpj-doctor-probe");

    if let Err(e) = backend.mount_overlay(&[lower], &upper, &work, &merged) {
        return Check::fail_with(label, format!("mount failed: {e}"), "Check dependency issues above");
    }

    let read_ok = std::fs::read_to_string(merged.join("probe.txt"))
        .map(|c| c == "fpj-doctor-probe")
        .unwrap_or(false);

    let _ = backend.unmount_overlay(&merged);

    if read_ok {
        Check::ok(format!("{label} passed"))
    } else {
        Check::fail_with(label, "mounted but could not read probe file", "Check mount logs")
    }
}

fn run_bind_smoke_test(backend: &dyn fpj::backend::MountBackend, label: &str) -> Check {
    let tmp = match tempfile::TempDir::new() {
        Ok(t) => t,
        Err(e) => return Check::fail_with(label, format!("temp dir: {e}"), ""),
    };

    let source = tmp.path().join("source");
    let target = tmp.path().join("target");
    for d in [&source, &target] {
        let _ = std::fs::create_dir_all(d);
    }
    let _ = std::fs::write(source.join("probe.txt"), "fpj-bind-probe");

    if let Err(e) = backend.bind_mount(&source, &target) {
        return Check::fail_with(label, format!("mount failed: {e}"), "Check dependency issues above");
    }

    let read_ok = std::fs::read_to_string(target.join("probe.txt"))
        .map(|c| c == "fpj-bind-probe")
        .unwrap_or(false);

    let _ = backend.unbind_mount(&target);

    if read_ok {
        Check::ok(format!("{label} passed"))
    } else {
        Check::fail_with(label, "mounted but could not read probe file", "Check mount logs")
    }
}

pub fn handle(json: bool) -> Result<()> {
    let overlay_deps = check_overlay_deps();
    let bind_deps = check_bind_deps();

    let overlay_deps_ok = overlay_deps.iter().all(|c| c.status == Status::Ok);
    let bind_deps_ok = bind_deps.iter().all(|c| c.status == Status::Ok);

    let fuse_backend = fpj::backend::create_backend_with(fpj::backend::BackendKind::Fuse);

    let mut overlay_checks = overlay_deps;
    if overlay_deps_ok {
        overlay_checks.push(run_smoke_test(fuse_backend.as_ref(), "FUSE overlay smoke test"));
    }

    let mut bind_checks = bind_deps;
    if bind_deps_ok {
        bind_checks.push(run_bind_smoke_test(fuse_backend.as_ref(), "FUSE bind smoke test"));
    }

    let kernel_deps = check_kernel_overlay_deps();
    let kernel_deps_ok = kernel_deps.iter().all(|c| c.status == Status::Ok);
    let mut kernel_checks = kernel_deps;
    if kernel_deps_ok {
        let kernel_backend = fpj::backend::create_backend_with(fpj::backend::BackendKind::Kernel);
        kernel_checks.push(run_smoke_test(kernel_backend.as_ref(), "Kernel overlay smoke test"));
        kernel_checks.push(run_bind_smoke_test(kernel_backend.as_ref(), "Kernel bind smoke test"));
    }

    let sections = vec![
        Section {
            title: "FUSE overlay filesystem".into(),
            checks: overlay_checks,
        },
        Section {
            title: "FUSE bind mounts".into(),
            checks: bind_checks,
        },
        Section {
            title: "Kernel overlayfs (for CRIU checkpoint/restore)".into(),
            checks: kernel_checks,
        },
    ];

    if json {
        print_json(&sections);
    } else {
        print_report(&sections);
    }

    Ok(())
}
