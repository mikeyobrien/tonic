mod common;

use std::fs;

/// Install with --copy, verify source is copied (not symlinked).
#[test]
fn install_copy_mode_copies_source() {
    let (dir, home) = common::isolated_tonic_home("install-copy-mode");

    // Create fixture project
    let project = dir.join("copy-pkg");
    let bin_dir = project.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(
        project.join("tonic.toml"),
        "[package]\nname = \"copy-pkg\"\n",
    )
    .unwrap();
    fs::write(bin_dir.join("copy-pkg"), "#!/bin/sh\necho copied\n").unwrap();

    // Install with --copy
    common::tonic_cmd(&["install", project.to_str().unwrap(), "--copy"])
        .env("TONIC_HOME", &home)
        .assert()
        .success()
        .stdout(predicates::prelude::predicate::str::contains("copy"));

    // Verify the package dir is a real directory, not a symlink
    let pkg_path = home.join("packages").join("copy-pkg");
    assert!(pkg_path.exists(), "package dir should exist");
    assert!(
        pkg_path.read_link().is_err(),
        "package dir should NOT be a symlink in copy mode"
    );

    // Verify tonic.toml was copied
    assert!(
        pkg_path.join("tonic.toml").exists(),
        "tonic.toml should be copied"
    );

    // Verify manifest records copy mode
    let manifest = fs::read_to_string(home.join("packages.toml")).unwrap();
    assert!(
        manifest.contains("symlink = false"),
        "manifest should record symlink = false for copy mode"
    );

    let _ = fs::remove_dir_all(&dir);
}
