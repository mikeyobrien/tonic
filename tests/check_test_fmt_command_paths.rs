use std::fs;
use std::path::PathBuf;
mod common;

#[test]
fn check_accepts_project_root_path_and_emits_ok_contract() {
    let fixture_root = write_project_fixture("check-command-path");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "."])
        .output()
        .expect("check command should execute");

    assert!(
        output.status.success(),
        "expected check command to succeed for project root path, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "check: ok\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

#[test]
fn test_accepts_project_root_path_and_emits_ok_contract() {
    let fixture_root = write_project_fixture("test-command-path");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "expected test command to succeed for project root path, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "test: ok\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

#[test]
fn fmt_accepts_project_root_path_and_emits_ok_contract() {
    let fixture_root = write_project_fixture("fmt-command-path");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["fmt", "."])
        .output()
        .expect("fmt command should execute");

    assert!(
        output.status.success(),
        "expected fmt command to succeed for project root path, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "fmt: ok\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

fn write_project_fixture(test_name: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write entry source file");

    fixture_root
}
