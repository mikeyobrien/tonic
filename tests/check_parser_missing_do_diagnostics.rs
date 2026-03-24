use std::fs;
mod common;

#[test]
fn check_reports_missing_module_do_with_opening_span() {
    let fixture_root = common::unique_fixture_root("check-missing-module-do");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_module_do.tn"),
        "defmodule Broken\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write missing module do fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_module_do.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0006] missing 'do' to start module 'Broken'; found DEF(def) instead. hint: add 'do' after 'defmodule Broken' to begin the module body"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_module_do.tn:1:1"),
        "unexpected parser diagnostic location: {stderr}"
    );
}

#[test]
fn check_reports_missing_if_do_with_opening_span() {
    let fixture_root = common::unique_fixture_root("check-missing-if-do");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_if_do.tn"),
        "defmodule Demo do\n  def run(flag) do\n    if flag\n      1\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write missing if do fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_if_do.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0006] missing 'do' to start if expression; found INT(1) instead. hint: add 'do' after the if condition to begin the then branch"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_if_do.tn:3:5"),
        "unexpected parser diagnostic location: {stderr}"
    );
}

#[test]
fn check_reports_missing_try_do_with_opening_span() {
    let fixture_root = common::unique_fixture_root("check-missing-try-do");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_try_do.tn"),
        "defmodule Demo do\n  def run() do\n    try\n      risky()\n    rescue\n      _ -> :error\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write missing try do fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_try_do.tn"])
        .output()
        .expect("check command should run");

    assert!(!output.status.success(), "expected check command to fail");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0006] missing 'do' to start try expression; found IDENT(risky) instead. hint: add 'do' after 'try' to begin the protected block"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains(" --> examples/missing_try_do.tn:3:5"),
        "unexpected parser diagnostic location: {stderr}"
    );
}
