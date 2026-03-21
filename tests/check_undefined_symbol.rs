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
    assert!(
        stderr.contains("--> examples/resolver_undefined_symbol.tn:3:5"),
        "expected filename:line:col location, got: {stderr}"
    );
    assert!(stderr.contains("3 |     missing()"));
}

#[test]
fn check_reports_local_call_typo_with_did_you_mean_hint() {
    let fixture_root = common::unique_fixture_root("check-undefined-symbol-local-typo");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("resolver_local_call_typo.tn"),
        "defmodule Demo do\n  def run() do\n    helpr()\n  end\n\n  def helper() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write local typo source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/resolver_local_call_typo.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for local call typo, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");

    assert!(
        stderr.contains(
            "error: [E1001] undefined symbol 'helpr' in Demo.run; did you mean `helper/0`?"
        ),
        "unexpected undefined-symbol hint diagnostic: {stderr}"
    );
    assert!(stderr.contains("--> examples/resolver_local_call_typo.tn:3:5"));
    assert!(stderr.contains("3 |     helpr()"));
}
