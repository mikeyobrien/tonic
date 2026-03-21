use std::fs;
mod common;

fn run_check_fixture(test_name: &str, fixture_name: &str, source: &str) -> String {
    let fixture_root = common::unique_fixture_root(test_name);
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(examples_dir.join(fixture_name), source)
        .expect("fixture setup should write typing mismatch fixture");

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
fn check_reports_bool_hint_for_not_operand() {
    let stderr = run_check_fixture(
        "check-type-mismatch-not-bool",
        "not_non_bool.tn",
        "defmodule Demo do\n  def run() do\n    not 1\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2001] type mismatch: expected bool, found int; hint: use a boolean expression here, for example `value != 0` or `is_nil(value)`"
    ));
    assert!(stderr.contains("--> examples/not_non_bool.tn:3:9"));
    assert!(stderr.contains("3 |     not 1"));
}

#[test]
fn check_reports_bool_hint_for_case_guard() {
    let stderr = run_check_fixture(
        "check-type-mismatch-case-guard",
        "case_guard_non_bool.tn",
        "defmodule Demo do\n  def run() do\n    case 1 do\n      value when 1 -> value\n      _ -> 0\n    end\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2001] type mismatch: expected bool, found int; hint: use a boolean expression here, for example `value != 0` or `is_nil(value)`"
    ));
    assert!(stderr.contains("--> examples/case_guard_non_bool.tn:4:18"));
    assert!(stderr.contains("4 |       value when 1 -> value"));
}

#[test]
fn check_reports_numeric_hint_for_bitwise_bool_operand() {
    let stderr = run_check_fixture(
        "check-type-mismatch-bitwise-bool",
        "bitwise_bool_operand_hint.tn",
        "defmodule Demo do\n  def run() do\n    true &&& 1\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2001] type mismatch: `&&&` requires ints on both sides, found bool on the left-hand side; hint: replace the boolean operand with an int value, or use `and`/`or` for boolean logic"
    ));
    assert!(stderr.contains("--> examples/bitwise_bool_operand_hint.tn:3:5"));
    assert!(stderr.contains("3 |     true &&& 1"));
}

#[test]
fn check_reports_atom_hint_for_host_call_key() {
    let stderr = run_check_fixture(
        "check-type-mismatch-host-call-key",
        "host_call_non_atom_key_hint.tn",
        "defmodule Demo do\n  def run() do\n    host_call(1, 2)\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2001] type mismatch: expected atom, found int; hint: pass an atom key as the first argument, for example `:sum_ints`"
    ));
    assert!(stderr.contains("--> examples/host_call_non_atom_key_hint.tn:3:15"));
    assert!(stderr.contains("3 |     host_call(1, 2)"));
}
