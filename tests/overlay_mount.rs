mod common;

use fpj::model::LayerSource;

use common::TestFixture;

#[test]
fn overlay_mount_unmount_cycle() {
    require!(common::can_use_fuse(), "FUSE not available");

    let mut f = TestFixture::new();
    let lower = f.mkdir("lower");
    f.write_file("lower/base-file.txt", "from base");
    let merged = f.mkdir("merged");

    {
        let engine = f.engine();

        engine
            .create_layer(
                "test-layer",
                LayerSource::Directory(lower.clone()),
                merged.clone(),
            )
            .unwrap();

        engine.create_layout("overlay-test").unwrap();
        engine
            .add_step(
                "overlay-test",
                fpj::model::MountStepDef::Layer("test-layer".to_string()),
            )
            .unwrap();

        engine.mount("overlay-test").unwrap();
    }

    let content = std::fs::read_to_string(merged.join("base-file.txt")).unwrap();
    assert_eq!(content, "from base");

    // Write to merged view goes to the internal upper layer
    std::fs::write(merged.join("new-file.txt"), "written").unwrap();

    {
        let engine = f.engine();
        engine.unmount("overlay-test").unwrap();

        assert!(!fpj::backend::create_backend().is_mounted(&merged).unwrap());
    }
}

#[test]
fn overlay_with_chained_layers() {
    require!(common::can_use_fuse(), "FUSE not available");
    require!(common::can_bind_mount(), "bind mount not available");

    let mut f = TestFixture::new();
    let base_dir = f.mkdir("base");
    f.write_file("base/file-a.txt", "from base");

    let merged_child = f.mkdir("merged-child");
    let merged_parent = f.root().join("merged-parent");

    {
        let engine = f.engine();

        engine
            .create_layer(
                "parent",
                LayerSource::Directory(base_dir.clone()),
                merged_parent,
            )
            .unwrap();

        engine.lock_layer("parent").unwrap();

        engine
            .create_layer(
                "child",
                LayerSource::Layer("parent".to_string()),
                merged_child.clone(),
            )
            .unwrap();

        engine.create_layout("chain-test").unwrap();
        engine
            .add_step(
                "chain-test",
                fpj::model::MountStepDef::Layer("child".to_string()),
            )
            .unwrap();

        engine.mount("chain-test").unwrap();
    }

    let content = std::fs::read_to_string(merged_child.join("file-a.txt")).unwrap();
    assert_eq!(content, "from base");

    {
        let engine = f.engine();
        engine.unmount("chain-test").unwrap();
    }
}
