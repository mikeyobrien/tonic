use std::fs;
mod common;

#[test]
fn run_executes_string_interpolation_and_prints_rendered_value() {
    let fixture_root = common::unique_fixture_root("run-string-interpolation");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("interpolation.tn"),
        "defmodule Demo do\n  def run() do\n    \"hello #{1 + 2} world #{:atom}\"\n  end\nend\n",
    )
    .expect("fixture setup should write interpolation source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/interpolation.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "\"hello 3 world atom\"\n");
}
