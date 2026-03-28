mod common;

use std::fs;

/// Install a package, run `tonic installed`, verify it appears in output.
#[test]
fn installed_lists_installed_package() {
    let (dir, home) = common::isolated_tonic_home("install-installed-list");

    // Create fixture project
    let project = dir.join("list-test");
    let bin_dir = project.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(
        project.join("tonic.toml"),
        "[package]\nname = \"list-test\"\n",
    )
    .unwrap();
    fs::write(bin_dir.join("list-test"), "#!/bin/sh\necho listed\n").unwrap();

    // Install
    common::tonic_cmd(&["install", project.to_str().unwrap()])
        .env("TONIC_HOME", &home)
        .assert()
        .success();

    // Run `tonic installed` and check output
    common::tonic_cmd(&["installed"])
        .env("TONIC_HOME", &home)
        .assert()
        .success()
        .stdout(predicates::prelude::predicate::str::contains("list-test"));

    let _ = fs::remove_dir_all(&dir);
}
