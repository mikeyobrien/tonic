use std::fs;
mod common;

#[test]
fn fmt_rewrites_source_and_is_idempotent() {
    let fixture_root = common::unique_fixture_root("fmt-rewrite-idempotent");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");

    let source_path = examples_dir.join("fmt_sample.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\ndef run() do\nif true do\n1\nelse\n2\nend\nend\nend\n",
    )
    .expect("fixture setup should write unformatted source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["fmt", "examples/fmt_sample.tn"])
        .output()
        .expect("fmt command should execute");

    assert!(
        output.status.success(),
        "expected fmt command to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "fmt: ok\n");

    let formatted = fs::read_to_string(&source_path).expect("formatted source should be readable");
    assert_eq!(
        formatted,
        "defmodule Demo do\n  def run() do\n    if true do\n      1\n    else\n      2\n    end\n  end\nend\n"
    );

    let second_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["fmt", "examples/fmt_sample.tn"])
        .output()
        .expect("second fmt command should execute");

    assert!(
        second_output.status.success(),
        "expected second fmt command to succeed, got status {:?} and stderr: {}",
        second_output.status.code(),
        String::from_utf8_lossy(&second_output.stderr)
    );

    let after_second_pass =
        fs::read_to_string(&source_path).expect("formatted source should still be readable");
    assert_eq!(after_second_pass, formatted);
}

#[test]
fn fmt_check_fails_when_source_needs_formatting() {
    let fixture_root = common::unique_fixture_root("fmt-check-fail");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");

    let source_path = examples_dir.join("fmt_check_sample.tn");
    let original_source = "defmodule Demo do\ndef run() do\n1\nend\nend\n";
    fs::write(&source_path, original_source).expect("fixture setup should write sample source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["fmt", "examples/fmt_check_sample.tn", "--check"])
        .output()
        .expect("fmt --check command should execute");

    assert!(
        !output.status.success(),
        "expected fmt --check command to fail when formatting is required, got status {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: formatting required for 1 file (run `tonic fmt <path>` to apply fixes)\n"
    );

    let content_after_check =
        fs::read_to_string(&source_path).expect("source should remain readable after --check");
    assert_eq!(content_after_check, original_source);
}

#[test]
fn fmt_check_succeeds_when_source_is_already_formatted() {
    let fixture_root = common::unique_fixture_root("fmt-check-ok");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");

    fs::write(
        examples_dir.join("formatted.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write pre-formatted source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["fmt", "examples/formatted.tn", "--check"])
        .output()
        .expect("fmt --check command should execute");

    assert!(
        output.status.success(),
        "expected fmt --check command to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "fmt: ok\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(stderr, "");
}

#[test]
fn fmt_rewrites_struct_syntax_without_mutating_expressions() {
    let fixture_root = common::unique_fixture_root("fmt-struct-syntax");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");

    let source_path = examples_dir.join("fmt_struct_syntax.tn");
    fs::write(
        &source_path,
        "defmodule User do\ndefstruct name: \"\", age: 0\ndef run(user) do\ncase %User{user | age: 43} do\n%User{name: name} ->\n%User{name: name}\n_ ->\n%User{}\nend\nend\nend\n",
    )
    .expect("fixture setup should write unformatted struct source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["fmt", "examples/fmt_struct_syntax.tn"])
        .output()
        .expect("fmt command should execute");

    assert!(
        output.status.success(),
        "expected fmt command to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let formatted = fs::read_to_string(&source_path).expect("formatted source should be readable");
    assert_eq!(
        formatted,
        "defmodule User do\n  defstruct name: \"\", age: 0\n  def run(user) do\n    case %User{user | age: 43} do\n      %User{name: name} ->\n        %User{name: name}\n      _ ->\n        %User{}\n    end\n  end\nend\n"
    );
}
