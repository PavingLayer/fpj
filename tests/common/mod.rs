use std::path::{Path, PathBuf};
use std::process::Command;

use fpj::backend::create_backend;
use fpj::backend::MountBackend;
use fpj::database::LayoutDatabase;
use fpj::engine::LayoutEngine;
use tempfile::TempDir;

/// Test fixture that provides a temporary database, directory tree, and engine.
pub struct TestFixture {
    pub dir: TempDir,
    pub db_path: PathBuf,
    engine: Option<LayoutEngine>,
}

impl TestFixture {
    pub fn new() -> Self {
        let dir = TempDir::new().expect("failed to create temp dir");
        let db_path = dir.path().join("test.db");
        Self {
            dir,
            db_path,
            engine: None,
        }
    }

    /// Get a reference to a LayoutEngine (lazily created with the real backend).
    pub fn engine(&mut self) -> &LayoutEngine {
        if self.engine.is_none() {
            let db = LayoutDatabase::open(&self.db_path).expect("failed to open test db");
            let backend = create_backend();
            self.engine = Some(LayoutEngine::new(db, backend));
        }
        self.engine.as_ref().unwrap()
    }

    /// Create an engine with a custom backend (consumes the fixture's db path).
    pub fn engine_with_backend(self, backend: Box<dyn MountBackend>) -> (TempDir, LayoutEngine) {
        let db = LayoutDatabase::open(&self.db_path).expect("failed to open test db");
        (self.dir, LayoutEngine::new(db, backend))
    }

    /// Open a fresh database connection (for direct DB tests).
    pub fn open_db(&self) -> LayoutDatabase {
        LayoutDatabase::open(&self.db_path).expect("failed to open test db")
    }

    pub fn root(&self) -> &Path {
        self.dir.path()
    }

    pub fn mkdir(&self, name: &str) -> PathBuf {
        let p = self.dir.path().join(name);
        std::fs::create_dir_all(&p).expect("failed to create dir");
        p
    }

    pub fn write_file(&self, rel_path: &str, content: &str) -> PathBuf {
        let p = self.dir.path().join(rel_path);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).expect("failed to create parent dirs");
        }
        std::fs::write(&p, content).expect("failed to write file");
        p
    }
}

// --- Capability detection ---

pub fn has_fuse_overlayfs() -> bool {
    Command::new("fuse-overlayfs")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn has_fusermount() -> bool {
    Command::new("fusermount")
        .arg("-V")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn can_use_fuse() -> bool {
    Path::new("/dev/fuse").exists() && has_fuse_overlayfs() && has_fusermount()
}

pub fn can_bind_mount() -> bool {
    #[cfg(target_os = "linux")]
    {
        nix::unistd::geteuid().is_root()
    }
    #[cfg(target_os = "windows")]
    {
        true
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("bindfs")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

#[macro_export]
macro_rules! require {
    ($check:expr, $msg:expr) => {
        if !$check {
            eprintln!("SKIP: {}", $msg);
            return;
        }
    };
}
