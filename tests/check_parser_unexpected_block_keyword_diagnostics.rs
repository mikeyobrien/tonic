use std::fs;
mod common;

#[test]
fn check_reports_stray_else_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-unexpected-else");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unexpected_else.tn"),
        "defmodule Demo do\n  def run() do\n    else\n  end\nend\n",
    )
    .expect("fixture setup should write unexpected else fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unexpected_else.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0005] unexpected 'else' without a matching block. hint: move 'else' inside an 'if', 'unless', or 'with' expression, or remove the extra 'else'"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/unexpected_else.tn:3:5"),
        "unexpected parser diagnostic location: {stderr}"
    );
}

#[test]
fn check_reports_stray_rescue_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-unexpected-rescue");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unexpected_rescue.tn"),
        "defmodule Demo do\n  def run() do\n    rescue\n  end\nend\n",
    )
    .expect("fixture setup should write unexpected rescue fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unexpected_rescue.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0005] unexpected 'rescue' without a matching 'try'. hint: move 'rescue' inside a 'try ... end' expression, add the missing 'try', or remove the extra 'rescue'"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/unexpected_rescue.tn:3:5"),
        "unexpected parser diagnostic location: {stderr}"
    );
}
