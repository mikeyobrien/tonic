use std::fs;
mod common;

#[test]
fn run_reports_manifest_validation_error_when_project_entry_missing() {
    let fixture_root = common::unique_fixture_root("run-manifest-validation");

    fs::create_dir_all(&fixture_root).expect("fixture setup should create project directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\n",
    )
    .expect("fixture setup should write tonic.toml");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: invalid tonic.toml: missing required key project.entry\n"
    );
}

#[test]
fn run_reports_error_when_tonic_toml_missing() {
    let fixture_root = common::unique_fixture_root("run-manifest-missing");
    std::fs::create_dir_all(&fixture_root).expect("fixture setup should create project directory");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("missing project manifest 'tonic.toml' at "));
}

#[test]
fn run_reports_error_when_tonic_toml_invalid() {
    let fixture_root = common::unique_fixture_root("run-manifest-invalid");
    std::fs::create_dir_all(&fixture_root).expect("fixture setup should create project directory");
    std::fs::write(fixture_root.join("tonic.toml"), "[project\n").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("invalid tonic.toml:"));
}

#[test]
fn run_reports_error_when_project_entry_empty() {
    let fixture_root = common::unique_fixture_root("run-manifest-empty-entry");
    std::fs::create_dir_all(&fixture_root).expect("fixture setup should create project directory");
    std::fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"   \"\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: invalid tonic.toml: project.entry cannot be empty\n"
    );
}

#[test]
fn run_reports_error_when_project_entry_does_not_exist() {
    let fixture_root = common::unique_fixture_root("run-manifest-entry-missing");
    std::fs::create_dir_all(&fixture_root).expect("fixture setup should create project directory");
    std::fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("error: project entry path 'src/main.tn' does not exist\n"));
}

#[test]
fn run_reports_error_when_project_entry_is_not_a_file() {
    let fixture_root = common::unique_fixture_root("run-manifest-entry-not-file");
    std::fs::create_dir_all(fixture_root.join("src/main.tn")).unwrap();
    std::fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("error: project entry path 'src/main.tn' is not a file\n"));
}

#[test]
fn run_reports_error_on_duplicate_module_definitions() {
    let fixture_root = common::unique_fixture_root("run-duplicate-modules");
    std::fs::create_dir_all(fixture_root.join("src")).unwrap();
    std::fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .unwrap();
    std::fs::write(fixture_root.join("src/main.tn"), "defmodule Demo do end").unwrap();
    std::fs::write(fixture_root.join("src/other.tn"), "defmodule Demo do end").unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E1003] duplicate module definition 'Demo'\n"
    );
}
