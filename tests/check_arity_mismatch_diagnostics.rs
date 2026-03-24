use std::fs;
mod common;

fn run_check_fixture(test_name: &str, fixture_name: &str, source: &str) -> String {
    let fixture_root = common::unique_fixture_root(test_name);
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(examples_dir.join(fixture_name), source)
        .expect("fixture setup should write arity mismatch fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", &format!("examples/{fixture_name}")])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check failure for {fixture_name}, got status {:?} with stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    String::from_utf8(output.stderr).expect("stderr should be utf8")
}

#[test]
fn check_reports_user_defined_arity_hint() {
    let stderr = run_check_fixture(
        "check-module-call-arity-mismatch",
        "module_call_arity_mismatch.tn",
        "defmodule Math do\n  def add(left, right) do\n    left + right\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.add(1)\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2002] arity mismatch for Math.add: expected 2 args, found 1; hint: call `Math.add/2`"
    ));
    assert!(stderr.contains("--> examples/module_call_arity_mismatch.tn:9:5"));
    assert!(stderr.contains("9 |     Math.add(1)"));
}

#[test]
fn check_reports_builtin_arity_hint() {
    let stderr = run_check_fixture(
        "check-builtin-arity-mismatch",
        "builtin_arity_mismatch.tn",
        "defmodule Demo do\n  def run() do\n    ok(1, 2)\n  end\nend\n",
    );

    assert!(
        stderr.contains(
            "error: [E2002] arity mismatch for ok: expected 1 arg, found 2; hint: call `ok/1`"
        ),
        "unexpected builtin arity diagnostic: {stderr}"
    );
    assert!(stderr.contains("--> examples/builtin_arity_mismatch.tn:3:5"));
    assert!(stderr.contains("3 |     ok(1, 2)"));
}

#[test]
fn check_reports_guard_builtin_arity_hint() {
    let stderr = run_check_fixture(
        "check-guard-builtin-arity-mismatch",
        "guard_builtin_arity_mismatch.tn",
        "defmodule Demo do\n  def choose(value) when is_integer(value, 1) do\n    value\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2002] arity mismatch for is_integer: expected 1 arg, found 2; hint: call `is_integer/1`"
    ));
    assert!(stderr.contains("--> examples/guard_builtin_arity_mismatch.tn:2:26"));
    assert!(stderr.contains("2 |   def choose(value) when is_integer(value, 1) do"));
}
