use std::fs;
use std::path::Path;
mod common;

#[test]
fn check_reports_parenthesized_call_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-call-paren",
        "call_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    Math.add(1 2)\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in call arguments; found INT(2) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: separate call arguments with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`Math.add(left, right)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_no_paren_call_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-call-no-paren",
        "call_no_paren_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    tuple 1 2\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in call arguments; found INT(2) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`tuple(left, right)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_function_param_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-function-params",
        "function_params_missing_comma.tn",
        "defmodule Demo do\n  def run(left right) do\n    left + right\n  end\nend\n",
    );

    assert!(
        stderr.contains(
            "[E0010] missing ',' in function parameter list; found IDENT(right) instead."
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`def run(left, right) do ... end`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_protocol_param_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-protocol-params",
        "protocol_params_missing_comma.tn",
        "defmodule Demo do\n  defprotocol Size do\n    def size(left right)\n  end\nend\n",
    );

    assert!(
        stderr.contains(
            "[E0010] missing ',' in protocol parameter list; found IDENT(right) instead."
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`def size(left, right)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

fn run_check(fixture_name: &str, file_name: &str, source: &str) -> String {
    let fixture_root = common::unique_fixture_root(fixture_name);
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    let source_path = examples_dir.join(file_name);
    fs::write(&source_path, source).expect("fixture setup should write invalid source file");

    let relative_path = relative_example_path(&source_path);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", relative_path.as_str()])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for malformed comma syntax"
    );

    String::from_utf8(output.stderr).expect("stderr should be utf8")
}

fn relative_example_path(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture file name should be utf8");
    format!("examples/{file_name}")
}
