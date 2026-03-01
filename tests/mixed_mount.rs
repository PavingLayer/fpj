mod common;

use fpj::model::{LayerSource, MountStepDef};

use common::TestFixture;

#[test]
fn interleaved_layer_and_binds() {
    require!(common::can_use_fuse(), "FUSE not available");
    require!(common::can_bind_mount(), "bind mount not available");

    let mut f = TestFixture::new();

    let lower = f.mkdir("lower");
    f.write_file("lower/base.txt", "base content");
    f.write_file("lower/config/default.cfg", "original config");

    let merged = f.mkdir("merged");

    let ext_config = f.mkdir("ext-config");
    f.write_file("ext-config/custom.cfg", "custom config");

    {
        let engine = f.engine();

        engine
            .create_layer(
                "ws-layer",
                LayerSource::Directory(lower.clone()),
                merged.clone(),
            )
            .unwrap();

        engine.create_layout("mixed-test").unwrap();
        engine
            .add_step("mixed-test", MountStepDef::Layer("ws-layer".to_string()))
            .unwrap();
        engine
            .add_step(
                "mixed-test",
                MountStepDef::Bind {
                    source: ext_config.clone(),
                    target: merged.join("config"),
                },
            )
            .unwrap();

        engine.mount("mixed-test").unwrap();
    }

    assert_eq!(
        std::fs::read_to_string(merged.join("base.txt")).unwrap(),
        "base content"
    );

    assert_eq!(
        std::fs::read_to_string(merged.join("config/custom.cfg")).unwrap(),
        "custom config"
    );
    assert!(!merged.join("config/default.cfg").exists());

    {
        let engine = f.engine();
        engine.unmount("mixed-test").unwrap();

        let backend = fpj::backend::create_backend();
        assert!(!backend.is_mounted(&merged.join("config")).unwrap());
        assert!(!backend.is_mounted(&merged).unwrap());
    }
}
