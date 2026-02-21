mod common;

use std::path::PathBuf;

use fpj::model::{Layout, MountStepDef};

use common::TestFixture;

#[test]
fn step_order_preserved_through_db_round_trip() {
    let f = TestFixture::new();
    let db = f.open_db();

    let layout = Layout {
        name: "ordered".to_string(),
        steps: vec![
            MountStepDef::Layer("layer-a".to_string()),
            MountStepDef::Bind {
                source: PathBuf::from("/bind-src-1"),
                target: PathBuf::from("/bind-tgt-1"),
            },
            MountStepDef::Layer("layer-b".to_string()),
            MountStepDef::Bind {
                source: PathBuf::from("/bind-src-2"),
                target: PathBuf::from("/bind-tgt-2"),
            },
            MountStepDef::Bind {
                source: PathBuf::from("/bind-src-3"),
                target: PathBuf::from("/bind-tgt-3"),
            },
        ],
    };

    db.save_layout(&layout).unwrap();
    let loaded = db.load_layout("ordered").unwrap();

    assert_eq!(loaded.steps.len(), 5);

    assert!(matches!(loaded.steps[0], MountStepDef::Layer(_)));
    assert!(matches!(loaded.steps[1], MountStepDef::Bind { .. }));
    assert!(matches!(loaded.steps[2], MountStepDef::Layer(_)));
    assert!(matches!(loaded.steps[3], MountStepDef::Bind { .. }));
    assert!(matches!(loaded.steps[4], MountStepDef::Bind { .. }));

    match &loaded.steps[1] {
        MountStepDef::Bind { source, .. } => {
            assert_eq!(*source, PathBuf::from("/bind-src-1"));
        }
        _ => unreachable!(),
    }
    match &loaded.steps[3] {
        MountStepDef::Bind { source, .. } => {
            assert_eq!(*source, PathBuf::from("/bind-src-2"));
        }
        _ => unreachable!(),
    }
}

#[test]
fn step_removal_preserves_remaining_order() {
    let f = TestFixture::new();
    let db = f.open_db();

    let mut layout = Layout::new("shrink".to_string());
    for i in 0..5 {
        layout.steps.push(MountStepDef::Bind {
            source: PathBuf::from(format!("/src/{i}")),
            target: PathBuf::from(format!("/tgt/{i}")),
        });
    }

    db.save_layout(&layout).unwrap();

    layout.remove_step(2).unwrap();
    db.save_layout(&layout).unwrap();

    let loaded = db.load_layout("shrink").unwrap();
    assert_eq!(loaded.steps.len(), 4);

    let sources: Vec<PathBuf> = loaded
        .steps
        .iter()
        .map(|s| match s {
            MountStepDef::Bind { source, .. } => source.clone(),
            _ => unreachable!(),
        })
        .collect();

    assert_eq!(
        sources,
        vec![
            PathBuf::from("/src/0"),
            PathBuf::from("/src/1"),
            PathBuf::from("/src/3"),
            PathBuf::from("/src/4"),
        ]
    );
}
