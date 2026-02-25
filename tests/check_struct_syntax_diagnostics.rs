use std::fs;
mod common;

#[test]
fn check_reports_undefined_struct_module() {
    let fixture_root = common::unique_fixture_root("check-undefined-struct-module");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("undefined_struct_module.tn"),
        "defmodule Demo do\n  def run() do\n    %Missing{name: \"A\"}\n  end\nend\n",
    )
    .expect("fixture setup should write undefined struct module source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/undefined_struct_module.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for undefined struct module"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("[E1004] undefined struct module 'Missing' in Demo.run"),
        "unexpected resolver diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unknown_struct_field_in_literal() {
    let fixture_root = common::unique_fixture_root("check-unknown-struct-field-literal");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unknown_struct_field_literal.tn"),
        "defmodule User do\n  defstruct name: \"\", age: 0\n\n  def run() do\n    %User{name: \"A\", agez: 42}\n  end\nend\n",
    )
    .expect("fixture setup should write unknown struct field source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unknown_struct_field_literal.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for unknown struct field"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("[E1005] unknown struct field 'agez' for User in User.run"),
        "unexpected resolver diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unknown_struct_field_in_update() {
    let fixture_root = common::unique_fixture_root("check-unknown-struct-field-update");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unknown_struct_field_update.tn"),
        "defmodule User do\n  defstruct name: \"\", age: 0\n\n  def run(user) do\n    %User{user | agez: 43}\n  end\nend\n",
    )
    .expect("fixture setup should write unknown struct update source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unknown_struct_field_update.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for unknown struct update field"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("[E1005] unknown struct field 'agez' for User in User.run"),
        "unexpected resolver diagnostic: {stderr}"
    );
}
