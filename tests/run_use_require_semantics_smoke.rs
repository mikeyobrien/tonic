use std::fs;

mod common;

#[test]
fn run_executes_use_scoped_import_fallback() {
    let fixture_root = common::unique_fixture_root("run-use-scoped-import-fallback");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("use_scoped_import.tn"),
        "defmodule Feature do\n  def helper() do\n    41\n  end\nend\n\ndefmodule Demo do\n  use Feature\n\n  def run() do\n    helper() + 1\n  end\nend\n",
    )
    .expect("fixture setup should write use semantics source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/use_scoped_import.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run invocation to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "42\n");
}

#[test]
fn check_reports_missing_required_module_target() {
    let fixture_root = common::unique_fixture_root("check-missing-required-module");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_required_module.tn"),
        "defmodule Demo do\n  require Missing\n\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write missing require source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_required_module.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check to fail for undefined require target, got status {:?} and stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "[E1011] required module 'Missing' is not defined for Demo; add the module or remove require"
        ),
        "unexpected require diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_missing_use_module_target() {
    let fixture_root = common::unique_fixture_root("check-missing-use-module");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_use_module.tn"),
        "defmodule Demo do\n  use Missing\n\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write missing use source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_use_module.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check to fail for undefined use target, got status {:?} and stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "[E1012] used module 'Missing' is not defined for Demo; add the module or remove use"
        ),
        "unexpected use diagnostic: {stderr}"
    );
}

#[test]
fn check_rejects_unsupported_require_and_use_options() {
    let fixture_root = common::unique_fixture_root("check-require-use-unsupported-options");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unsupported_require_option.tn"),
        "defmodule Demo do\n  require Logger, as: L\n\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write unsupported require option source file");
    fs::write(
        examples_dir.join("unsupported_use_option.tn"),
        "defmodule Demo do\n  use Feature, only: [helper: 0]\n\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write unsupported use option source file");

    let require_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unsupported_require_option.tn"])
        .output()
        .expect("check command should run for require option fixture");

    assert!(
        !require_output.status.success(),
        "expected check to fail for unsupported require options"
    );

    let require_stderr = String::from_utf8(require_output.stderr).expect("stderr should be utf8");
    assert!(
        require_stderr
            .contains("unsupported require option 'as'; remove options from require for now"),
        "unexpected require option error: {require_stderr}"
    );

    let use_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unsupported_use_option.tn"])
        .output()
        .expect("check command should run for use option fixture");

    assert!(
        !use_output.status.success(),
        "expected check to fail for unsupported use options"
    );

    let use_stderr = String::from_utf8(use_output.stderr).expect("stderr should be utf8");
    assert!(
        use_stderr.contains("unsupported use option 'only'; remove options from use for now"),
        "unexpected use option error: {use_stderr}"
    );
}
