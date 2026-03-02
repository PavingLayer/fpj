mod common;

use std::path::PathBuf;

use fpj::model::{LayerRole, LayerSource};

use common::TestFixture;

fn abs(p: &str) -> PathBuf {
    if cfg!(windows) {
        PathBuf::from(format!("C:\\{p}"))
    } else {
        PathBuf::from(format!("/{p}"))
    }
}

#[test]
fn lock_and_unlock_layer_via_engine() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "lock-test",
                LayerSource::Directory(abs("tmp/lower")),
                abs("tmp/mp"),
            )
            .unwrap();

        engine.lock_layer("lock-test").unwrap();
        let layer = engine.get_layer("lock-test").unwrap();
        assert_eq!(layer.role, LayerRole::Locked);

        engine.unlock_layer("lock-test").unwrap();
        let layer = engine.get_layer("lock-test").unwrap();
        assert_eq!(layer.role, LayerRole::Writable);
    }
}

#[test]
fn lock_already_locked_fails() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "double-lock",
                LayerSource::Directory(abs("tmp/lower")),
                abs("tmp/mp"),
            )
            .unwrap();

        engine.lock_layer("double-lock").unwrap();
        let result = engine.lock_layer("double-lock");
        assert!(result.is_err());
    }
}

#[test]
fn unlock_already_writable_fails() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "unlock-wr",
                LayerSource::Directory(abs("tmp/lower")),
                abs("tmp/mp"),
            )
            .unwrap();

        let result = engine.unlock_layer("unlock-wr");
        assert!(result.is_err());
    }
}

#[test]
fn create_layer_referencing_unlocked_base_fails() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "base-layer",
                LayerSource::Directory(abs("tmp/lower")),
                abs("tmp/mp1"),
            )
            .unwrap();

        let result = engine.create_layer(
            "child-layer",
            LayerSource::Layer("base-layer".to_string()),
            abs("tmp/mp2"),
        );
        assert!(result.is_err());
    }
}

#[test]
fn create_layer_referencing_locked_base_succeeds() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "base-layer",
                LayerSource::Directory(abs("tmp/lower")),
                abs("tmp/mp1"),
            )
            .unwrap();

        engine.lock_layer("base-layer").unwrap();

        engine
            .create_layer(
                "child-layer",
                LayerSource::Layer("base-layer".to_string()),
                abs("tmp/mp2"),
            )
            .unwrap();

        let child = engine.get_layer("child-layer").unwrap();
        assert_eq!(child.role, LayerRole::Writable);
    }
}

#[test]
fn chain_resolution_produces_correct_lower_dirs() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "grandparent",
                LayerSource::Directory(abs("data/base")),
                abs("tmp/mp-gp"),
            )
            .unwrap();

        engine.lock_layer("grandparent").unwrap();

        engine
            .create_layer(
                "parent",
                LayerSource::Layer("grandparent".to_string()),
                abs("tmp/mp-p"),
            )
            .unwrap();

        engine.lock_layer("parent").unwrap();

        engine
            .create_layer(
                "child",
                LayerSource::Layer("parent".to_string()),
                abs("tmp/mp-c"),
            )
            .unwrap();

        let lower_dirs = engine.resolve_lower_dirs("child").unwrap();

        assert_eq!(lower_dirs.len(), 3);

        let parent = engine.get_layer("parent").unwrap();
        let grandparent = engine.get_layer("grandparent").unwrap();

        assert_eq!(lower_dirs[0], parent.upper_dir);
        assert_eq!(lower_dirs[1], grandparent.upper_dir);
        assert_eq!(lower_dirs[2], abs("data/base"));
    }
}

#[test]
fn siblings_sharing_same_base_resolve_independently() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "shared-base",
                LayerSource::Directory(abs("data/base")),
                abs("tmp/mp-base"),
            )
            .unwrap();

        engine.lock_layer("shared-base").unwrap();

        engine
            .create_layer(
                "sibling-a",
                LayerSource::Layer("shared-base".to_string()),
                abs("tmp/mp-a"),
            )
            .unwrap();

        engine
            .create_layer(
                "sibling-b",
                LayerSource::Layer("shared-base".to_string()),
                abs("tmp/mp-b"),
            )
            .unwrap();

        let dirs_a = engine.resolve_lower_dirs("sibling-a").unwrap();
        let dirs_b = engine.resolve_lower_dirs("sibling-b").unwrap();

        let shared_base = engine.get_layer("shared-base").unwrap();

        assert_eq!(dirs_a.len(), 2);
        assert_eq!(dirs_b.len(), 2);
        assert_eq!(dirs_a[0], shared_base.upper_dir);
        assert_eq!(dirs_b[0], shared_base.upper_dir);
        assert_eq!(dirs_a[1], abs("data/base"));
        assert_eq!(dirs_b[1], abs("data/base"));
        assert_eq!(dirs_a, dirs_b);

        let a = engine.get_layer("sibling-a").unwrap();
        let b = engine.get_layer("sibling-b").unwrap();
        assert_ne!(a.upper_dir, b.upper_dir);
    }
}

#[test]
fn chain_resolution_fails_on_unlocked_base() {
    let mut f = TestFixture::new();

    {
        let engine = f.engine();

        engine
            .create_layer(
                "base",
                LayerSource::Directory(abs("data/base")),
                abs("tmp/mp1"),
            )
            .unwrap();

        engine.lock_layer("base").unwrap();

        engine
            .create_layer(
                "middle",
                LayerSource::Layer("base".to_string()),
                abs("tmp/mp2"),
            )
            .unwrap();

        let result = engine.resolve_lower_dirs("middle");
        assert!(result.is_ok());

        engine.lock_layer("middle").unwrap();
        engine
            .create_layer(
                "child",
                LayerSource::Layer("middle".to_string()),
                abs("tmp/mp3"),
            )
            .unwrap();

        engine.unlock_layer("middle").unwrap();

        let result = engine.resolve_lower_dirs("child");
        assert!(result.is_err());
    }
}
