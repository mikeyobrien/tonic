mod common;

use std::fs;

/// Install then uninstall, verify shim removed and manifest entry cleared.
#[test]
fn install_then_uninstall_cleans_up() {
    let (dir, home) = common::isolated_tonic_home("install-uninstall");

    // Create fixture project
    let project = dir.join("removable");
    let bin_dir = project.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(
        project.join("tonic.toml"),
        "[package]\nname = \"removable\"\n",
    )
    .unwrap();
    fs::write(bin_dir.join("removable"), "#!/bin/sh\necho hi\n").unwrap();

    // Install
    common::tonic_cmd(&["install", project.to_str().unwrap()])
        .env("TONIC_HOME", &home)
        .assert()
        .success();

    let shim = home.join("bin").join("removable");
    assert!(shim.exists(), "shim should exist after install");

    // Uninstall
    common::tonic_cmd(&["uninstall", "removable"])
        .env("TONIC_HOME", &home)
        .assert()
        .success()
        .stdout(predicates::prelude::predicate::str::contains(
            "Uninstalled package 'removable'",
        ));

    // Verify shim removed
    assert!(!shim.exists(), "shim should be removed after uninstall");

    // Verify manifest entry cleared
    let manifest = fs::read_to_string(home.join("packages.toml")).unwrap();
    assert!(
        !manifest.contains("removable"),
        "packages.toml should not contain removable after uninstall"
    );

    let _ = fs::remove_dir_all(&dir);
}
