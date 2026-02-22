use std::fs;
mod common;

#[test]
fn check_reports_deterministic_error_for_question_on_non_result_expression() {
    let fixture_root = common::unique_fixture_root("check-question-non-result");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("result_non_result_question.tn"),
        "defmodule Demo do\n  def value() do\n    1\n  end\n\n  def run() do\n    value()?\n  end\nend\n",
    )
    .expect("fixture setup should write non-result question source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/result_non_result_question.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for non-result ? usage, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert_eq!(
        stderr,
        "error: [E3001] ? operator requires Result value, found int at offset 74\n"
    );
}
