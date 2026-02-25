use std::fs;
mod common;

#[test]
fn check_reports_deterministic_undefined_symbol_error_code() {
    let fixture_root = common::unique_fixture_root("check-undefined-symbol");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_undefined_symbol.tn"),
        "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n",
    )
    .expect("fixture setup should write undefined symbol source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_undefined_symbol.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for undefined symbol, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert!(stderr.contains("error: [E1001] undefined symbol 'missing' in Demo.run"));
    assert!(stderr.contains("--> line 3, column 5"));
    assert!(stderr.contains("3 |     missing()"));
}
