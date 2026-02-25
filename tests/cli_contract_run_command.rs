//! CLI Contract Tests: run command
//!
//! These tests define the contract for the `tonic run` command:
//! - Usage errors (missing args, invalid args) → EXIT_USAGE (64)
//! - Runtime errors (file not found, type errors) → EXIT_FAILURE (1)
//! - Success → EXIT_OK (0)

use std::fs;

/// Test: run with no arguments returns usage error
#[test]
fn run_no_args_is_usage_error() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .arg("run")
        .output()
        .expect("run command should execute");

    // No args should return usage error (exit 64)
    assert!(
        !output.status.success(),
        "expected usage error for missing path, got status {:?}",
        output.status.code()
    );
    assert_eq!(
        output.status.code(),
        Some(64),
        "expected exit 64 for usage error, got {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: missing required <path>"),
        "expected usage error about missing path, got: {}",
        stderr
    );
}

/// Test: run with empty string path returns file not found (runtime error)
#[test]
fn run_empty_path_is_runtime_error() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .args(["run", ""])
        .output()
        .expect("run command should execute");

    // Empty path is treated as file not found - runtime error (exit 1)
    assert!(
        !output.status.success(),
        "expected failure for empty path, got status {:?}",
        output.status.code()
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for file not found, got {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error:") && stderr.contains("No such file"),
        "expected file not found error, got: {}",
        stderr
    );
}

/// Test: run with non-existent file returns file not found (runtime error)
#[test]
fn run_nonexistent_file_is_runtime_error() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .args(["run", "nonexistent.ex"])
        .output()
        .expect("run command should execute");

    // Non-existent file is runtime error (exit 1)
    assert!(
        !output.status.success(),
        "expected failure for nonexistent file, got status {:?}",
        output.status.code()
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for file not found, got {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error:") && stderr.contains("No such file"),
        "expected file not found error, got: {}",
        stderr
    );
}

/// Test: run with valid file succeeds
#[test]
fn run_valid_file_succeeds() {
    // Use a simple valid fixture - valid Tonic code must be in a defmodule
    let fixture_dir = std::env::temp_dir().join("tonic_test_run_valid");
    fs::create_dir_all(&fixture_dir).ok();
    fs::write(
        fixture_dir.join("test.tn"),
        r#"defmodule Demo do
  def run() do
    1 + 1
  end
end
"#,
    )
    .ok();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_dir)
        .args(["run", "test.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected success for valid file, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "2");

    fs::remove_dir_all(fixture_dir).ok();
}

/// Test: run with syntax error returns runtime/type error
#[test]
fn run_syntax_error_is_runtime_error() {
    let fixture_dir = std::env::temp_dir().join("tonic_test_run_syntax");
    fs::create_dir_all(&fixture_dir).ok();
    fs::write(fixture_dir.join("bad.ex"), "def invalid_syntax").ok();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_dir)
        .args(["run", "bad.ex"])
        .output()
        .expect("run command should execute");

    // Syntax errors are runtime errors (exit 1)
    assert!(
        !output.status.success(),
        "expected failure for syntax error, got status {:?}",
        output.status.code()
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit 1 for syntax error, got {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error:"),
        "expected error in stderr, got: {}",
        stderr
    );

    fs::remove_dir_all(fixture_dir).ok();
}
