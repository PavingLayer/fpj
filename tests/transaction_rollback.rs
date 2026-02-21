mod common;

use std::path::PathBuf;

use fpj::backend::MountBackend;
use fpj::engine::LayoutEngine;
use fpj::error::{LayerfsError, Result};
use fpj::model::{Layer, LayerRole, LayerSource, Layout, MountStepDef};
use fpj::operations::MountTransaction;

use common::TestFixture;

/// A backend that succeeds for the first N steps, then fails.
struct FailingBackend {
    fail_at: usize,
    call_count: std::cell::Cell<usize>,
    undo_log: std::cell::RefCell<Vec<String>>,
}

impl FailingBackend {
    fn new(fail_at: usize) -> Self {
        Self {
            fail_at,
            call_count: std::cell::Cell::new(0),
            undo_log: std::cell::RefCell::new(Vec::new()),
        }
    }

    fn undo_log(&self) -> Vec<String> {
        self.undo_log.borrow().clone()
    }
}

impl MountBackend for FailingBackend {
    fn mount_overlay(
        &self,
        _lower_dirs: &[PathBuf],
        _upper_dir: &std::path::Path,
        _work_dir: &std::path::Path,
        _mount_point: &std::path::Path,
    ) -> Result<()> {
        let count = self.call_count.get();
        self.call_count.set(count + 1);
        if count >= self.fail_at {
            return Err(LayerfsError::Backend("injected failure".into()));
        }
        Ok(())
    }

    fn unmount_overlay(&self, mount_point: &std::path::Path) -> Result<()> {
        self.undo_log
            .borrow_mut()
            .push(format!("unmount_overlay:{}", mount_point.display()));
        Ok(())
    }

    fn bind_mount(&self, _source: &std::path::Path, _target: &std::path::Path) -> Result<()> {
        let count = self.call_count.get();
        self.call_count.set(count + 1);
        if count >= self.fail_at {
            return Err(LayerfsError::Backend("injected failure".into()));
        }
        Ok(())
    }

    fn unbind_mount(&self, target: &std::path::Path) -> Result<()> {
        self.undo_log
            .borrow_mut()
            .push(format!("unbind:{}", target.display()));
        Ok(())
    }

    fn is_mounted(&self, _path: &std::path::Path) -> Result<bool> {
        Ok(false)
    }

    fn ensure_writable_in_overlay(&self, _path: &std::path::Path) -> Result<()> {
        Ok(())
    }
}

#[test]
fn rollback_on_third_step_failure() {
    let f = TestFixture::new();
    let db = f.open_db();

    let layer = Layer {
        name: "test-layer".to_string(),
        source: LayerSource::Directory(PathBuf::from("/base")),
        mount_point: f.root().join("mp"),
        role: LayerRole::Writable,
        upper_dir: f.root().join("upper"),
        work_dir: f.root().join("work"),
    };
    db.create_layer(&layer).unwrap();

    let layout = Layout {
        name: "rollback-test".to_string(),
        steps: vec![
            MountStepDef::Layer("test-layer".to_string()),
            MountStepDef::Bind {
                source: PathBuf::from("/src1"),
                target: PathBuf::from("/tgt1"),
            },
            MountStepDef::Bind {
                source: PathBuf::from("/src2"),
                target: PathBuf::from("/tgt2"),
            },
        ],
    };
    db.save_layout(&layout).unwrap();

    let backend = FailingBackend::new(2);
    let db2 = f.open_db();
    let engine = LayoutEngine::new(db2, Box::new(FailingBackend::new(999)));
    let db3 = f.open_db();

    let mut tx = MountTransaction::new(&layout, &db3, &engine, &backend);
    let result = tx.execute();

    assert!(result.is_err());

    let undo = backend.undo_log();
    assert_eq!(undo.len(), 2);
    assert_eq!(undo[0], "unbind:/tgt1");
    assert!(undo[1].starts_with("unmount_overlay:"));
}

#[test]
fn no_rollback_when_first_step_fails() {
    let f = TestFixture::new();
    let db = f.open_db();

    let layer = Layer {
        name: "test-layer".to_string(),
        source: LayerSource::Directory(PathBuf::from("/base")),
        mount_point: f.root().join("mp"),
        role: LayerRole::Writable,
        upper_dir: f.root().join("upper"),
        work_dir: f.root().join("work"),
    };
    db.create_layer(&layer).unwrap();

    let layout = Layout {
        name: "first-fail".to_string(),
        steps: vec![MountStepDef::Layer("test-layer".to_string())],
    };
    db.save_layout(&layout).unwrap();

    let backend = FailingBackend::new(0);
    let db2 = f.open_db();
    let engine = LayoutEngine::new(db2, Box::new(FailingBackend::new(999)));
    let db3 = f.open_db();

    let mut tx = MountTransaction::new(&layout, &db3, &engine, &backend);
    let result = tx.execute();

    assert!(result.is_err());
    assert!(backend.undo_log().is_empty());
}

#[test]
fn all_steps_succeed_no_rollback() {
    let f = TestFixture::new();
    let db = f.open_db();

    let layer = Layer {
        name: "test-layer".to_string(),
        source: LayerSource::Directory(PathBuf::from("/base")),
        mount_point: f.root().join("mp"),
        role: LayerRole::Writable,
        upper_dir: f.root().join("upper"),
        work_dir: f.root().join("work"),
    };
    db.create_layer(&layer).unwrap();

    let layout = Layout {
        name: "success".to_string(),
        steps: vec![
            MountStepDef::Layer("test-layer".to_string()),
            MountStepDef::Bind {
                source: PathBuf::from("/s"),
                target: PathBuf::from("/t"),
            },
        ],
    };
    db.save_layout(&layout).unwrap();

    let backend = FailingBackend::new(999);
    let db2 = f.open_db();
    let engine = LayoutEngine::new(db2, Box::new(FailingBackend::new(999)));
    let db3 = f.open_db();

    let mut tx = MountTransaction::new(&layout, &db3, &engine, &backend);
    let result = tx.execute();

    assert!(result.is_ok());
    assert!(backend.undo_log().is_empty());
}
