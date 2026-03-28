mod common;

use std::fs;

/// Install from directory without tonic.toml → error.
#[test]
fn install_dir_without_tonic_toml_errors() {
    let (dir, home) = common::isolated_tonic_home("install-no-tonic-toml");

    // Create a directory with no tonic.toml
    let project = dir.join("no-manifest");
    fs::create_dir_all(&project).unwrap();

    common::tonic_cmd(&["install", project.to_str().unwrap()])
        .env("TONIC_HOME", &home)
        .assert()
        .code(1)
        .stderr(predicates::prelude::predicate::str::contains("no tonic.toml"));

    let _ = fs::remove_dir_all(&dir);
}
