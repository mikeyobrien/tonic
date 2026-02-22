use std::fs;
mod common;

#[test]
fn run_executes_alias_and_imported_calls() {
    let fixture_root = common::unique_fixture_root("run-module-forms-alias-import");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_module_forms.tn"),
        "defmodule Math do\n  def helper() do\n    7\n  end\nend\n\ndefmodule Demo do\n  alias Math, as: M\n  import Math\n  @doc \"run docs\"\n\n  def run() do\n    M.helper() + helper()\n  end\nend\n",
    )
    .expect("fixture setup should write module forms source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_module_forms.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "14\n");
}

#[test]
fn check_rejects_unsupported_module_form_option() {
    let fixture_root = common::unique_fixture_root("check-module-forms-invalid-option");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("invalid_module_form.tn"),
        "defmodule Demo do\n  alias Math, via: M\n\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write invalid module form source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/invalid_module_form.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check to fail for unsupported module form option, got status {:?} and stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: unsupported alias option 'via'; supported syntax: alias Module, as: Name at offset 32\n"
    );
}
