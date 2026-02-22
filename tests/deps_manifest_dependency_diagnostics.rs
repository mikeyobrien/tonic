use std::fs;
use std::path::PathBuf;
mod common;

#[test]
fn deps_lock_rejects_dependency_without_path_or_git_source() {
    let project_root = setup_project(
        "deps-lock-rejects-missing-source",
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nbroken = { rev = \"abc123\" }\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["deps", "lock"])
        .output()
        .expect("deps lock command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: invalid tonic.toml: dependency 'broken' must specify either a string 'path' or both string 'git' and 'rev'\n"
    );
}

#[test]
fn deps_lock_rejects_path_dependency_with_non_string_path_value() {
    let project_root = setup_project(
        "deps-lock-rejects-non-string-path",
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nbroken = { path = 42 }\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_root)
        .args(["deps", "lock"])
        .output()
        .expect("deps lock command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: invalid tonic.toml: path dependency 'broken' has non-string 'path' value\n"
    );
}

fn setup_project(test_name: &str, manifest_source: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let project_root = fixture_root.join("app");
    let src_dir = project_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create project src directory");

    fs::write(project_root.join("tonic.toml"), manifest_source)
        .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    project_root
}
