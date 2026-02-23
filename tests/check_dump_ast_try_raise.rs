use std::fs;
mod common;

#[test]
fn check_dump_ast_matches_try_raise_contract() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-try-raise");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_raise_smoke.tn"),
        "defmodule Demo do
  def run() do
    try do
      raise(:boom)
    rescue
      _ -> :ok
    end
  end
end
",
    )
    .expect("fixture setup should write parser smoke source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/try_raise_smoke.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected try/raise to parse successfully, got status {:?} and stderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#""kind":"try""#),
        "AST output should contain Try expression"
    );
    assert!(
        stdout.contains(r#""kind":"raise""#),
        "AST output should contain Raise expression"
    );
}

#[test]
fn check_try_catch_unsupported_diagnostic() {
    let fixture_root = common::unique_fixture_root("check-try-catch-unsupported");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_catch.tn"),
        "defmodule Demo do
  def run() do
    try do
      :ok
    catch
      _ -> :ok
    end
  end
end
",
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/try_catch.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("catch is out of scope for now"),
        "should report unsupported catch"
    );
}

#[test]
fn check_try_after_unsupported_diagnostic() {
    let fixture_root = common::unique_fixture_root("check-try-after-unsupported");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_after.tn"),
        "defmodule Demo do
  def run() do
    try do
      :ok
    after
      :ok
    end
  end
end
",
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/try_after.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("after is out of scope for now"),
        "should report unsupported after"
    );
}
