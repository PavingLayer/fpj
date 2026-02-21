mod common;

use std::path::PathBuf;

use fpj::model::{Layout, MountStepDef};

use common::TestFixture;

#[test]
fn create_and_list_layouts() {
    let f = TestFixture::new();
    let db = f.open_db();
    db.create_layout("alpha").unwrap();
    db.create_layout("beta").unwrap();

    let names = db.list_layouts().unwrap();
    assert_eq!(names, vec!["alpha", "beta"]);
}

#[test]
fn create_duplicate_layout_fails() {
    let f = TestFixture::new();
    let db = f.open_db();
    db.create_layout("dup").unwrap();
    let result = db.create_layout("dup");
    assert!(result.is_err());
}

#[test]
fn remove_layout() {
    let f = TestFixture::new();
    let db = f.open_db();
    db.create_layout("gone").unwrap();
    db.remove_layout("gone").unwrap();

    let names = db.list_layouts().unwrap();
    assert!(names.is_empty());
}

#[test]
fn remove_nonexistent_layout_fails() {
    let f = TestFixture::new();
    let db = f.open_db();
    let result = db.remove_layout("nope");
    assert!(result.is_err());
}

#[test]
fn save_and_load_layout_with_steps() {
    let f = TestFixture::new();
    let db = f.open_db();
    let layout = Layout {
        name: "test-ws".to_string(),
        steps: vec![
            MountStepDef::Layer("my-layer".to_string()),
            MountStepDef::Bind {
                source: PathBuf::from("/source/config"),
                target: PathBuf::from("/merged/config"),
            },
            MountStepDef::Bind {
                source: PathBuf::from("/source/ext"),
                target: PathBuf::from("/merged/ext"),
            },
        ],
    };

    db.save_layout(&layout).unwrap();
    let loaded = db.load_layout("test-ws").unwrap();

    assert_eq!(loaded.name, "test-ws");
    assert_eq!(loaded.steps.len(), 3);

    match &loaded.steps[0] {
        MountStepDef::Layer(name) => assert_eq!(name, "my-layer"),
        other => panic!("Expected Layer, got {other:?}"),
    }

    match &loaded.steps[1] {
        MountStepDef::Bind { source, target } => {
            assert_eq!(*source, PathBuf::from("/source/config"));
            assert_eq!(*target, PathBuf::from("/merged/config"));
        }
        other => panic!("Expected Bind, got {other:?}"),
    }

    match &loaded.steps[2] {
        MountStepDef::Bind { source, target } => {
            assert_eq!(*source, PathBuf::from("/source/ext"));
            assert_eq!(*target, PathBuf::from("/merged/ext"));
        }
        other => panic!("Expected Bind, got {other:?}"),
    }
}

#[test]
fn load_nonexistent_layout_fails() {
    let f = TestFixture::new();
    let db = f.open_db();
    let result = db.load_layout("nope");
    assert!(result.is_err());
}

#[test]
fn layout_exists_check() {
    let f = TestFixture::new();
    let db = f.open_db();
    assert!(!db.layout_exists("x").unwrap());
    db.create_layout("x").unwrap();
    assert!(db.layout_exists("x").unwrap());
}

#[test]
fn save_layout_replaces_previous_steps() {
    let f = TestFixture::new();
    let db = f.open_db();

    let layout_v1 = Layout {
        name: "evolving".to_string(),
        steps: vec![MountStepDef::Bind {
            source: PathBuf::from("/a"),
            target: PathBuf::from("/b"),
        }],
    };
    db.save_layout(&layout_v1).unwrap();

    let layout_v2 = Layout {
        name: "evolving".to_string(),
        steps: vec![
            MountStepDef::Bind {
                source: PathBuf::from("/x"),
                target: PathBuf::from("/y"),
            },
            MountStepDef::Bind {
                source: PathBuf::from("/p"),
                target: PathBuf::from("/q"),
            },
        ],
    };
    db.save_layout(&layout_v2).unwrap();

    let loaded = db.load_layout("evolving").unwrap();
    assert_eq!(loaded.steps.len(), 2);

    match &loaded.steps[0] {
        MountStepDef::Bind { source, .. } => assert_eq!(*source, PathBuf::from("/x")),
        other => panic!("Expected Bind, got {other:?}"),
    }
}

#[test]
fn remove_layout_cascades_to_steps() {
    let f = TestFixture::new();
    let db = f.open_db();

    let layout = Layout {
        name: "cascade".to_string(),
        steps: vec![MountStepDef::Bind {
            source: PathBuf::from("/s"),
            target: PathBuf::from("/t"),
        }],
    };
    db.save_layout(&layout).unwrap();
    db.remove_layout("cascade").unwrap();

    assert!(db.load_layout("cascade").is_err());
}
