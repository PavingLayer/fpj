mod common;

use fpj::model::{LayerSource, MountStepDef};

use common::TestFixture;

#[test]
fn restore_layout_from_database() {
    require!(common::can_use_fuse(), "FUSE not available");

    let mut f = TestFixture::new();

    let lower = f.mkdir("lower");
    f.write_file("lower/data.txt", "persistent");
    let merged = f.mkdir("merged");

    {
        let engine = f.engine();

        engine
            .create_layer("persist-layer", LayerSource::Directory(lower.clone()), merged.clone())
            .unwrap();

        engine.create_layout("persist-test").unwrap();
        engine
            .add_step(
                "persist-test",
                MountStepDef::Layer("persist-layer".to_string()),
            )
            .unwrap();

        engine.mount("persist-test").unwrap();
    }

    assert_eq!(
        std::fs::read_to_string(merged.join("data.txt")).unwrap(),
        "persistent"
    );

    {
        let engine = f.engine();
        engine.unmount("persist-test").unwrap();
        engine.restore(Some("persist-test")).unwrap();
    }

    assert_eq!(
        std::fs::read_to_string(merged.join("data.txt")).unwrap(),
        "persistent"
    );

    {
        let engine = f.engine();
        engine.unmount("persist-test").unwrap();
    }
}
