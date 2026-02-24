use std::fs;
mod common;

#[test]
fn check_reports_deterministic_try_body_typing_error() {
    let fixture_root = common::unique_fixture_root("check-try-body-typing");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("try_body_typing.tn"),
        "defmodule Demo do
  def run() do
    try do
      missing()
    rescue
      _ -> :ok
    end
  end
end
",
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/try_body_typing.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure, got status {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E1001] undefined symbol 'missing' in Demo.run
"
    );
}

#[test]
fn check_reports_deterministic_raise_arg_typing_error() {
    let fixture_root = common::unique_fixture_root("check-raise-arg-typing");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("raise_arg_typing.tn"),
        "defmodule Demo do
  def run() do
    raise(missing())
  end
end
",
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/raise_arg_typing.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure, got status {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E1001] undefined symbol 'missing' in Demo.run
"
    );
}
