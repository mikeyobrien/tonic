mod common;

use predicates::prelude::*;
use std::fs;

/// Install with existing shim from different package; verify --force overwrites and no-flag errors.
#[test]
fn install_force_overwrites_conflicting_shim() {
    let (dir, home) = common::isolated_tonic_home("install-force-overwrite");

    // Create first project
    let project_a = dir.join("pkg-a");
    let bin_a = project_a.join("bin");
    fs::create_dir_all(&bin_a).unwrap();
    fs::write(
        project_a.join("tonic.toml"),
        "[package]\nname = \"pkg-a\"\n",
    )
    .unwrap();
    fs::write(bin_a.join("shared-bin"), "#!/bin/sh\necho a\n").unwrap();

    // Create second project with same binary name
    let project_b = dir.join("pkg-b");
    let bin_b = project_b.join("bin");
    fs::create_dir_all(&bin_b).unwrap();
    fs::write(
        project_b.join("tonic.toml"),
        "[package]\nname = \"pkg-b\"\n",
    )
    .unwrap();
    fs::write(bin_b.join("shared-bin"), "#!/bin/sh\necho b\n").unwrap();

    // Install first project
    common::tonic_cmd(&["install", project_a.to_str().unwrap()])
        .env("TONIC_HOME", &home)
        .assert()
        .success();

    // Install second project WITHOUT --force should fail
    common::tonic_cmd(&["install", project_b.to_str().unwrap()])
        .env("TONIC_HOME", &home)
        .assert()
        .code(1)
        .stderr(predicate::str::contains("already installed by package"));

    // Install second project WITH --force should succeed
    common::tonic_cmd(&["install", project_b.to_str().unwrap(), "--force"])
        .env("TONIC_HOME", &home)
        .assert()
        .success();

    let _ = fs::remove_dir_all(&dir);
}
