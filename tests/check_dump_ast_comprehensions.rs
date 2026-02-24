use std::fs;
mod common;

#[test]
fn check_dump_ast_supports_single_generator_for_comprehension() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-comprehensions");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_comprehension.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2, 3) do\n      x + 1\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for comprehension source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/for_comprehension.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let body = &json["modules"][0]["functions"][0]["body"];

    assert_eq!(body["kind"], "for");
    let first_generator = &body["generators"][0];
    assert_eq!(first_generator[0]["kind"], "bind");
    assert_eq!(first_generator[0]["name"], "x");
    assert_eq!(first_generator[1]["kind"], "call");
    assert_eq!(first_generator[1]["callee"], "list");
    assert_eq!(body["body"]["kind"], "binary");
    assert_eq!(body["body"]["op"], "plus");
}

#[test]
fn check_dump_ast_supports_multi_generator_for_comprehension() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-multi-comprehensions");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_multi_comprehension.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1), y <- list(2) do\n      x + y\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write multi for comprehension source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/for_multi_comprehension.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let body = &json["modules"][0]["functions"][0]["body"];

    assert_eq!(body["kind"], "for");
    let first_generator = &body["generators"][0];
    assert_eq!(first_generator[0]["kind"], "bind");
    assert_eq!(first_generator[0]["name"], "x");
    assert_eq!(first_generator[1]["kind"], "call");
    assert_eq!(first_generator[1]["callee"], "list");
    let second_generator = &body["generators"][1];
    assert_eq!(second_generator[0]["kind"], "bind");
    assert_eq!(second_generator[0]["name"], "y");
    assert_eq!(second_generator[1]["kind"], "call");
    assert_eq!(second_generator[1]["callee"], "list");
}

#[test]
fn check_rejects_unsupported_for_options_with_deterministic_hint() {
    let fixture_root = common::unique_fixture_root("check-for-options");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_options.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), reduce: [] do\n      x\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write unsupported for option source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/for_options.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for unsupported for options, got status {:?} and stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unsupported for option 'reduce'; remove options from for for now"),
        "expected deterministic unsupported option diagnostic, stderr was: {stderr}"
    );
}
