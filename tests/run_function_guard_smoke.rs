use std::fs;
mod common;

#[test]
fn run_executes_guarded_function_clause_when_guard_is_true() {
    let fixture_root = common::unique_fixture_root("run-function-guard-true");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_function_guard_true.tn"),
        "defmodule Demo do\n  def choose(value) when value == 7 do\n    value\n  end\n\n  def run() do\n    choose(7)\n  end\nend\n",
    )
    .expect("fixture setup should write guarded function source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_function_guard_true.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "7\n");
}

#[test]
fn run_reports_deterministic_guard_clause_failures() {
    let fixture_root = common::unique_fixture_root("run-function-guard-false");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_function_guard_false.tn"),
        "defmodule Demo do\n  def choose(value) when value == 7 do\n    value\n  end\n\n  def run() do\n    choose(8)\n  end\nend\n",
    )
    .expect("fixture setup should write guarded function source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_function_guard_false.tn"])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: no function clause matching Demo.choose at offset 43\n"
    );
}
