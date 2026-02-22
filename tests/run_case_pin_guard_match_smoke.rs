use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_case_with_pin_patterns_and_guards() {
    let fixture_root = unique_fixture_root("run-case-pin-guard");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_case_pin_guard.tn"),
        "defmodule Demo do\n  def classify(expected) do\n    case list(expected, 8) do\n      [^expected, value] when value == 8 -> value\n      _ -> 0\n    end\n  end\n\n  def run() do\n    classify(7)\n  end\nend\n",
    )
    .expect("fixture setup should write pin/guard source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_case_pin_guard.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "8\n");
}

#[test]
fn run_executes_match_operator_with_destructuring_patterns() {
    let fixture_root = unique_fixture_root("run-match-destructure");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_match_destructure.tn"),
        "defmodule Demo do\n  def run() do\n    [head, _] = list(9, 4)\n  end\nend\n",
    )
    .expect("fixture setup should write match source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_match_destructure.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[9, 4]\n");
}

#[test]
fn run_reports_deterministic_bad_match_diagnostics() {
    let fixture_root = unique_fixture_root("run-match-mismatch");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_match_mismatch.tn"),
        "defmodule Demo do\n  def run() do\n    [1, 2] = list(1, 3)\n  end\nend\n",
    )
    .expect("fixture setup should write mismatch source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_match_mismatch.tn"])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: no match of right hand side value: [1, 3] at offset 37\n"
    );
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}
