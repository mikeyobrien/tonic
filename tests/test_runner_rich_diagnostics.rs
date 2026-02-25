use serde_json::Value;
use std::fs;
use std::path::PathBuf;
mod common;

#[test]
fn test_project_root_discovers_and_runs_test_files() {
    let fixture_root = common::unique_fixture_root("test-runner-project-root");
    let src_dir = fixture_root.join("src");
    let tests_dir = fixture_root.join("tests");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&tests_dir).expect("fixture setup should create tests directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write main module");
    fs::write(
        tests_dir.join("math_test.tn"),
        "defmodule MathTest do\n  def test_add() do\n    1 + 1\n  end\nend\n",
    )
    .expect("fixture setup should write test module");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "expected test command success, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout,
        "test MathTest.test_add ... ok\ntest result: ok. 1 passed; 0 failed; 1 total\n"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

#[test]
fn test_returns_non_zero_and_deterministic_summary_when_failures_exist() {
    let fixture_root = write_single_test_file(
        "test-runner-failure-summary",
        "failing_test.tn",
        "defmodule FailingTest do\n  def test_ok() do\n    42\n  end\n\n  def test_fail() do\n    err(7)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1 when at least one test fails"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("test FailingTest.test_fail ... FAILED"));
    assert!(stdout.contains("error: runtime returned err(7)"));
    assert!(stdout.contains("test FailingTest.test_ok ... ok"));
    assert!(stdout.contains("test result: FAILED. 1 passed; 1 failed; 2 total"));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

#[test]
fn test_file_target_mode_executes_tests_from_explicit_file_path() {
    let fixture_root = write_single_test_file(
        "test-runner-file-target",
        "manual_suite.tn",
        "defmodule ManualSuite do\n  def test_manual_case() do\n    :ok\n  end\nend\n",
    );

    let file_path = fixture_root.join("manual_suite.tn");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["test", file_path.to_str().expect("utf8 file path")])
        .output()
        .expect("test command should execute");

    assert!(
        output.status.success(),
        "expected explicit file path to pass"
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("test ManualSuite.test_manual_case ... ok"));
    assert!(stdout.contains("test result: ok. 1 passed; 0 failed; 1 total"));
}

#[test]
fn test_supports_machine_readable_json_output() {
    let fixture_root = write_single_test_file(
        "test-runner-json-output",
        "json_mode_test.tn",
        "defmodule JsonModeTest do\n  def test_alpha() do\n    1\n  end\n\n  def test_beta() do\n    err(:boom)\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["test", ".", "--format", "json"])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let report: Value = serde_json::from_str(&stdout).expect("json output should parse");

    assert_eq!(report["status"], "failed");
    assert_eq!(report["total"], 2);
    assert_eq!(report["passed"], 1);
    assert_eq!(report["failed"], 1);

    let results = report["results"]
        .as_array()
        .expect("results should be array");
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["id"], "JsonModeTest.test_alpha");
    assert_eq!(results[0]["status"], "passed");
    assert_eq!(results[1]["id"], "JsonModeTest.test_beta");
    assert_eq!(results[1]["status"], "failed");
    assert_eq!(results[1]["error"], "runtime returned err(:boom)");
}

#[test]
fn check_diagnostics_include_line_column_and_source_snippet() {
    let fixture_root = common::unique_fixture_root("check-rich-resolver-diagnostics");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_error.tn"),
        "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n",
    )
    .expect("fixture setup should write resolver fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_error.tn"])
        .output()
        .expect("check command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("[E1001] undefined symbol 'missing' in Demo.run"));
    assert!(stderr.contains("--> line 3, column 5"));
    assert!(stderr.contains("3 |     missing()"));
    assert!(stderr.contains("|     ^"));
}

#[test]
fn test_command_surfaces_rich_source_diagnostics_for_frontend_errors() {
    let fixture_root = write_single_test_file(
        "test-rich-diag-front-end",
        "invalid_test.tn",
        "defmodule InvalidTest do\n  def test_bad() do\n    %{1 2}\n  end\nend\n",
    );

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["test", "."])
        .output()
        .expect("test command should execute");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("expected map fat arrow `=>`, found INT(2)"));
    assert!(stderr.contains("--> line 3, column"));
    assert!(stderr.contains("3 |     %{1 2}"));
}

fn write_single_test_file(test_name: &str, file_name: &str, source: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);

    fs::write(fixture_root.join(file_name), source).expect("fixture setup should write test file");

    fixture_root
}
