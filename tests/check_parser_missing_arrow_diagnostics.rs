use std::fs;
mod common;

#[test]
fn check_reports_missing_case_arrow_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-missing-case-arrow");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_case_arrow.tn"),
        "defmodule Demo do\n  def run(value) do\n    case value do\n      :ok value\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write missing case arrow fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_case_arrow.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0007] missing '->' in case branch; found IDENT(value) instead. hint: add '->' after the case pattern before the branch body"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_case_arrow.tn:4:11"),
        "unexpected parser diagnostic location: {stderr}"
    );
}

#[test]
fn check_reports_missing_rescue_arrow_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-missing-rescue-arrow");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_rescue_arrow.tn"),
        "defmodule Demo do\n  def run() do\n    try do\n      risky()\n    rescue\n      Demo.Error :error\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write missing rescue arrow fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_rescue_arrow.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0007] missing '->' in try rescue clause; found ATOM(error) instead. hint: add '->' after the rescue pattern before the clause body"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_rescue_arrow.tn:6:18"),
        "unexpected parser diagnostic location: {stderr}"
    );
}

#[test]
fn check_reports_missing_anonymous_fn_arrow_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-missing-fn-arrow");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_fn_arrow.tn"),
        "defmodule Demo do\n  def run() do\n    fn value value + 1 end\n  end\nend\n",
    )
    .expect("fixture setup should write missing anonymous fn arrow fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_fn_arrow.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0007] missing '->' in anonymous function clause; found IDENT(value) instead. hint: add '->' between the anonymous function parameters and clause body"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_fn_arrow.tn:3:14"),
        "unexpected parser diagnostic location: {stderr}"
    );
}
