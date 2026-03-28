mod common;

use std::fs;

/// Install from a local fixture project, verify shim exists and packages.toml entry created.
#[test]
fn install_local_path_creates_shim_and_manifest_entry() {
    let (dir, home) = common::isolated_tonic_home("install-local-path");

    // Create a fixture project with tonic.toml and bin/
    let project = dir.join("my-tool");
    let bin_dir = project.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();
    fs::write(
        project.join("tonic.toml"),
        "[package]\nname = \"my-tool\"\n",
    )
    .unwrap();
    fs::write(bin_dir.join("my-tool"), "#!/bin/sh\necho hello\n").unwrap();

    // Run install
    common::tonic_cmd(&["install", project.to_str().unwrap()])
        .env("TONIC_HOME", &home)
        .assert()
        .success()
        .stdout(predicates::prelude::predicate::str::contains(
            "Installed package 'my-tool'",
        ));

    // Verify shim exists
    let shim = home.join("bin").join("my-tool");
    assert!(shim.exists(), "shim should exist at {}", shim.display());

    // Verify shim is executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&shim).unwrap().permissions().mode();
        assert!(mode & 0o111 != 0, "shim should be executable");
    }

    // Verify packages.toml entry
    let manifest_path = home.join("packages.toml");
    assert!(manifest_path.exists(), "packages.toml should exist");
    let manifest_content = fs::read_to_string(&manifest_path).unwrap();
    assert!(
        manifest_content.contains("my-tool"),
        "packages.toml should contain my-tool entry"
    );
    assert!(
        manifest_content.contains("source = \"path\""),
        "entry should have source = path"
    );

    let _ = fs::remove_dir_all(&dir);
}
