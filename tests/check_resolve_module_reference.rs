use std::fs;
mod common;

#[test]
fn check_resolves_module_qualified_call_across_modules() {
    let fixture_root = common::unique_fixture_root("check-resolve-module-reference");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_module_reference.tn"),
        "defmodule Math do\n  def helper() do\n    1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n",
    )
    .expect("fixture setup should write module reference source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_module_reference.tn"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected check success for module-qualified call, got status {:?} with stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert_eq!(stdout, "");
    assert_eq!(stderr, "");
}
