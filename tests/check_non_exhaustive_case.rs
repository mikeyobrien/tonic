use std::fs;
mod common;

#[test]
fn check_reports_non_exhaustive_case_when_wildcard_branch_is_missing() {
    let fixture_root = common::unique_fixture_root("check-non-exhaustive-case");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("non_exhaustive_case.tn"),
        "defmodule Demo do\n  def run() do\n    case value() do\n      :ok -> 1\n    end\n  end\n\n  def value() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write non-exhaustive case source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/non_exhaustive_case.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for non-exhaustive case expression, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert_eq!(
        stderr,
        "error: [E3002] non-exhaustive case expression: missing wildcard branch at offset 37\n"
    );
}
