use std::fs;
mod common;

fn run_check_fixture(test_name: &str, fixture_name: &str, source: &str) -> String {
    let fixture_root = common::unique_fixture_root(test_name);
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(examples_dir.join(fixture_name), source)
        .expect("fixture setup should write numeric mismatch fixture");

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
fn check_reports_numeric_hint_for_string_range_bound() {
    let stderr = run_check_fixture(
        "check-numeric-hint-range-string",
        "range_string_bound_hint.tn",
        "defmodule Demo do\n  def run() do\n    1..\"2\"\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2001] type mismatch: `..` requires int bounds, found string on the right-hand side; hint: convert the string bound to an int first, for example `String.to_integer(value)`"
    ));
    assert!(stderr.contains("--> examples/range_string_bound_hint.tn:3:8"));
    assert!(stderr.contains("3 |     1..\"2\""));
}

#[test]
fn check_reports_numeric_hint_for_nil_bitwise_not_operand() {
    let stderr = run_check_fixture(
        "check-numeric-hint-bitwise-not-nil",
        "bitwise_not_nil_operand_hint.tn",
        "defmodule Demo do\n  def run() do\n    ~~~nil\n  end\nend\n",
    );

    assert!(stderr.contains(
        "error: [E2001] type mismatch: `~~~` requires an int operand, found nil; hint: replace `nil` with an int before applying `~~~`"
    ));
    assert!(stderr.contains("--> examples/bitwise_not_nil_operand_hint.tn:3:8"));
    assert!(stderr.contains("3 |     ~~~nil"));
}
