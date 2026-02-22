use std::fs;
mod common;

#[test]
fn run_propagates_err_result_and_exits_with_failure() {
    let fixture_root = common::unique_fixture_root("run-result-propagation");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_result_err.tn"),
        "defmodule Demo do\n  def fail() do\n    err(7)\n  end\n\n  def run() do\n    fail()?\n  end\nend\n",
    )
    .expect("fixture setup should write result propagation source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_result_err.tn"])
        .output()
        .expect("run command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "error: runtime returned err(7)\n");
}
