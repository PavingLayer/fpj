mod common;

use fpj::model::MountStepDef;

use common::TestFixture;

#[test]
fn bind_mount_unmount_cycle() {
    require!(common::can_bind_mount(), "bind mount not available");

    let mut f = TestFixture::new();

    let source = f.mkdir("source-dir");
    f.write_file("source-dir/hello.txt", "bound content");
    let target = f.mkdir("target-dir");

    {
        let engine = f.engine();

        engine.create_layout("bind-test").unwrap();
        engine
            .add_step(
                "bind-test",
                MountStepDef::Bind {
                    source: source.clone(),
                    target: target.clone(),
                },
            )
            .unwrap();

        engine.mount("bind-test").unwrap();
    }

    let content = std::fs::read_to_string(target.join("hello.txt")).unwrap();
    assert_eq!(content, "bound content");

    {
        let engine = f.engine();
        engine.unmount("bind-test").unwrap();
        assert!(!fpj::backend::create_backend().is_mounted(&target).unwrap());
    }
}
