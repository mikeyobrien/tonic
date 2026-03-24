use std::fs;
mod common;

#[test]
fn check_reports_missing_module_end_with_opening_span() {
    let fixture_root = common::unique_fixture_root("check-missing-module-end");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_module_end.tn"),
        "defmodule Broken do\n  def run() do\n    1\n  end\n",
    )
    .expect("fixture setup should write missing module fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_module_end.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0003] unexpected end of file: missing 'end' to close module 'Broken'. hint: add 'end' to finish module 'Broken'"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_module_end.tn:1:1"),
        "unexpected parser diagnostic location: {stderr}"
    );
}

#[test]
fn check_reports_missing_function_end_with_opening_span() {
    let fixture_root = common::unique_fixture_root("check-missing-function-end");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_function_end.tn"),
        "defmodule Demo do\n  def run() do\n    1\n",
    )
    .expect("fixture setup should write missing function fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_function_end.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0003] unexpected end of file: missing 'end' to close function 'run'. hint: add 'end' to finish function 'run'"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_function_end.tn:2:3"),
        "unexpected parser diagnostic location: {stderr}"
    );
}

#[test]
fn check_reports_missing_if_end_with_opening_span() {
    let fixture_root = common::unique_fixture_root("check-missing-if-end");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_if_end.tn"),
        "defmodule Demo do\n  def run(flag) do\n    if flag do\n      1\n",
    )
    .expect("fixture setup should write missing if fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_if_end.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0003] unexpected end of file: missing 'end' to close if expression. hint: add 'end' to finish if expression"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_if_end.tn:3:5"),
        "unexpected parser diagnostic location: {stderr}"
    );
}
