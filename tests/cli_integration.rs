use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn fpj_cmd(db_path: &std::path::Path) -> Command {
    let mut cmd = assert_cmd::cargo_bin_cmd!("fpj");
    cmd.arg("--db").arg(db_path);
    cmd
}

fn p(tmp: &TempDir, name: &str) -> String {
    tmp.path().join(name).to_string_lossy().into_owned()
}

#[test]
fn layout_create_list_show_remove() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.db");

    fpj_cmd(&db)
        .args(["layout", "create", "my-layout"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created layout 'my-layout'"));

    fpj_cmd(&db)
        .args(["layout", "create", "my-layout"])
        .assert()
        .failure();

    fpj_cmd(&db)
        .args(["layout", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my-layout"));

    fpj_cmd(&db)
        .args(["layout", "show", "my-layout"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Steps (0)"));

    fpj_cmd(&db)
        .args(["layout", "remove", "my-layout"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed layout 'my-layout'"));

    fpj_cmd(&db)
        .args(["layout", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No layouts defined"));
}

#[test]
fn layer_create_list_show_remove() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.db");

    fpj_cmd(&db)
        .args([
            "layer",
            "create",
            "base-img",
            "--source",
            &p(&tmp, "base"),
            "--mount-point",
            &p(&tmp, "merged"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created layer 'base-img'"));

    fpj_cmd(&db)
        .args([
            "layer",
            "create",
            "base-img",
            "--source",
            &p(&tmp, "base"),
            "--mount-point",
            &p(&tmp, "merged2"),
        ])
        .assert()
        .failure();

    fpj_cmd(&db)
        .args(["layer", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("base-img"));

    fpj_cmd(&db)
        .args(["layer", "show", "base-img"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Layer: base-img")
                .and(predicate::str::contains("Source:"))
                .and(predicate::str::contains("Role:        writable")),
        );

    fpj_cmd(&db)
        .args(["layer", "remove", "base-img"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed layer 'base-img'"));

    fpj_cmd(&db)
        .args(["layer", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No layers defined"));
}

#[test]
fn layer_lock_unlock() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.db");

    fpj_cmd(&db)
        .args([
            "layer",
            "create",
            "lk",
            "--source",
            &p(&tmp, "base"),
            "--mount-point",
            &p(&tmp, "mp"),
        ])
        .assert()
        .success();

    fpj_cmd(&db)
        .args(["layer", "lock", "lk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Locked layer 'lk'"));

    // Lock again should fail
    fpj_cmd(&db)
        .args(["layer", "lock", "lk"])
        .assert()
        .failure();

    fpj_cmd(&db)
        .args(["layer", "unlock", "lk"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Unlocked layer 'lk'"));
}

#[test]
fn step_add_layer_and_bind() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.db");

    fpj_cmd(&db)
        .args([
            "layer",
            "create",
            "ws-layer",
            "--source",
            &p(&tmp, "base"),
            "--mount-point",
            &p(&tmp, "merged"),
        ])
        .assert()
        .success();

    fpj_cmd(&db)
        .args(["layout", "create", "ws"])
        .assert()
        .success();

    // Add layer step
    fpj_cmd(&db)
        .args(["step", "add-layer", "ws", "--layer", "ws-layer"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added layer step"));

    // Add bind step
    fpj_cmd(&db)
        .args([
            "step",
            "add-bind",
            "ws",
            "--source",
            &p(&tmp, "src"),
            "--target",
            &p(&tmp, "tgt"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added bind step"));

    // List steps
    fpj_cmd(&db)
        .args(["step", "list", "ws"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("[0] layer @ws-layer")
                .and(predicate::str::contains("[1] bind")),
        );

    // Show layout with steps
    fpj_cmd(&db)
        .args(["layout", "show", "ws"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Steps (2)"));

    // Remove step 0
    fpj_cmd(&db)
        .args(["step", "remove", "ws", "--position", "0"])
        .assert()
        .success();

    fpj_cmd(&db)
        .args(["layout", "show", "ws"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Steps (1)"));
}

#[test]
fn layer_reference_with_at_syntax() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.db");

    fpj_cmd(&db)
        .args([
            "layer",
            "create",
            "base",
            "--source",
            &p(&tmp, "base-dir"),
            "--mount-point",
            &p(&tmp, "mp1"),
        ])
        .assert()
        .success();

    fpj_cmd(&db)
        .args(["layer", "lock", "base"])
        .assert()
        .success();

    // Create child layer referencing base with @ syntax
    fpj_cmd(&db)
        .args([
            "layer",
            "create",
            "child",
            "--source",
            "@base",
            "--mount-point",
            &p(&tmp, "mp2"),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created layer 'child'"));

    fpj_cmd(&db)
        .args(["layer", "show", "child"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Source:      @base"));
}

#[test]
fn reject_relative_paths() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.db");

    fpj_cmd(&db)
        .args(["layout", "create", "rp"])
        .assert()
        .success();

    // Relative path in bind source should fail
    fpj_cmd(&db)
        .args([
            "step",
            "add-bind",
            "rp",
            "--source",
            "relative/path",
            "--target",
            &p(&tmp, "tgt"),
        ])
        .assert()
        .failure();

    // Relative path in layer source should fail
    fpj_cmd(&db)
        .args([
            "layer",
            "create",
            "bad-layer",
            "--source",
            "relative/lower",
            "--mount-point",
            &p(&tmp, "merged"),
        ])
        .assert()
        .failure();
}

#[test]
fn status_json_output() {
    let tmp = TempDir::new().unwrap();
    let db = tmp.path().join("test.db");

    fpj_cmd(&db)
        .args(["layout", "create", "js"])
        .assert()
        .success();

    fpj_cmd(&db)
        .args([
            "step",
            "add-bind",
            "js",
            "--source",
            &p(&tmp, "src"),
            "--target",
            &p(&tmp, "tgt"),
        ])
        .assert()
        .success();

    let output = fpj_cmd(&db)
        .args(["status", "js", "--json"])
        .output()
        .unwrap();

    let stdout = String::from_utf8(output.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(parsed["name"], "js");
    assert!(parsed["steps"].is_array());
}
