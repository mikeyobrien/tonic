use std::fs;
mod common;

#[test]
fn check_reports_unexpected_arrow_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-unexpected-arrow");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unexpected_arrow.tn"),
        "defmodule Demo do\n  def run() do\n    value -> value + 1\n  end\nend\n",
    )
    .expect("fixture setup should write unexpected arrow fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unexpected_arrow.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0004] unexpected '->' outside a valid branch. hint: use 'fn ... -> ... end' for anonymous functions, or move '->' into a branch inside case/cond/with/for/try"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/unexpected_arrow.tn:3:11"),
        "unexpected parser diagnostic location: {stderr}"
    );
}
