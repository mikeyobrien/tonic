mod common;

/// Install from non-existent path → meaningful error, non-zero exit.
#[test]
fn install_missing_source_path_errors() {
    let (dir, home) = common::isolated_tonic_home("install-missing-source");

    common::tonic_cmd(&["install", "/tmp/tonic-nonexistent-path-12345"])
        .env("TONIC_HOME", &home)
        .assert()
        .code(1)
        .stderr(predicates::prelude::predicate::str::contains("cannot resolve path"));

    let _ = std::fs::remove_dir_all(&dir);
}
