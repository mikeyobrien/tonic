use std::fs;
mod common;

fn run_tonic_source(test_name: &str, source: &str) -> String {
    let fixture_root = common::unique_fixture_root(test_name);
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("test.tn"),
        source,
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/test.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation for {test_name}, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).expect("stdout should be utf8")
}

// ============================================================
// Feature 2a: Strict equality (===, !==)
// ============================================================

#[test]
fn strict_eq_integers_same_value_returns_true() {
    let source = "defmodule Demo do\n  def run() do\n    1 === 1\n  end\nend\n";
    let stdout = run_tonic_source("strict-eq-int-true", source);
    assert_eq!(stdout, "true\n");
}

#[test]
fn strict_eq_integer_and_float_returns_false() {
    let source = "defmodule Demo do\n  def run() do\n    1 === 1.0\n  end\nend\n";
    let stdout = run_tonic_source("strict-eq-int-float", source);
    assert_eq!(stdout, "false\n");
}

#[test]
fn strict_bang_eq_different_types_returns_true() {
    let source = "defmodule Demo do\n  def run() do\n    1 !== 1.0\n  end\nend\n";
    let stdout = run_tonic_source("strict-bang-eq-diff-types", source);
    assert_eq!(stdout, "true\n");
}

#[test]
fn strict_eq_atoms_same_value_returns_true() {
    let source = "defmodule Demo do\n  def run() do\n    :ok === :ok\n  end\nend\n";
    let stdout = run_tonic_source("strict-eq-atoms", source);
    assert_eq!(stdout, "true\n");
}

#[test]
fn strict_eq_fixture_produces_expected_output() {
    let source = "defmodule Demo do\n  def run() do\n    {1 === 1, {1 === 1.0, {1 !== 1.0, :ok === :ok}}}\n  end\nend\n";
    let stdout = run_tonic_source("strict-eq-fixture", source);
    assert_eq!(stdout, "{true, {false, {true, true}}}\n");
}

// ============================================================
// Feature 2b: div and rem
// ============================================================

#[test]
fn div_integer_division_truncates_toward_zero() {
    let source = "defmodule Demo do\n  def run() do\n    div(10, 3)\n  end\nend\n";
    let stdout = run_tonic_source("div-basic", source);
    assert_eq!(stdout, "3\n");
}

#[test]
fn rem_returns_remainder() {
    let source = "defmodule Demo do\n  def run() do\n    rem(10, 3)\n  end\nend\n";
    let stdout = run_tonic_source("rem-basic", source);
    assert_eq!(stdout, "1\n");
}

#[test]
fn div_rem_fixture_produces_expected_output() {
    let source = "defmodule Demo do\n  def run() do\n    {div(10, 3), rem(10, 3)}\n  end\nend\n";
    let stdout = run_tonic_source("div-rem-fixture", source);
    assert_eq!(stdout, "{3, 1}\n");
}

// ============================================================
// Feature 2c: not in
// ============================================================

#[test]
fn not_in_value_not_in_list_returns_true() {
    let source = "defmodule Demo do\n  def run() do\n    5 not in [1, 2, 3]\n  end\nend\n";
    let stdout = run_tonic_source("not-in-true", source);
    assert_eq!(stdout, "true\n");
}

#[test]
fn not_in_value_in_list_returns_false() {
    let source = "defmodule Demo do\n  def run() do\n    2 not in [1, 2, 3]\n  end\nend\n";
    let stdout = run_tonic_source("not-in-false", source);
    assert_eq!(stdout, "false\n");
}

#[test]
fn not_in_fixture_produces_expected_output() {
    let source = "defmodule Demo do\n  def run() do\n    {5 not in [1, 2, 3], 2 not in [1, 2, 3]}\n  end\nend\n";
    let stdout = run_tonic_source("not-in-fixture", source);
    assert_eq!(stdout, "{true, false}\n");
}

// ============================================================
// Feature 2d: Bitwise operators
// ============================================================

#[test]
fn bitwise_and_produces_correct_result() {
    let source = "defmodule Demo do\n  def run() do\n    5 &&& 3\n  end\nend\n";
    let stdout = run_tonic_source("bitwise-and", source);
    assert_eq!(stdout, "1\n");
}

#[test]
fn bitwise_or_produces_correct_result() {
    let source = "defmodule Demo do\n  def run() do\n    5 ||| 3\n  end\nend\n";
    let stdout = run_tonic_source("bitwise-or", source);
    assert_eq!(stdout, "7\n");
}

#[test]
fn bitwise_xor_produces_correct_result() {
    let source = "defmodule Demo do\n  def run() do\n    5 ^^^ 6\n  end\nend\n";
    let stdout = run_tonic_source("bitwise-xor", source);
    assert_eq!(stdout, "3\n");
}

#[test]
fn bitwise_not_produces_correct_result() {
    let source = "defmodule Demo do\n  def run() do\n    ~~~5\n  end\nend\n";
    let stdout = run_tonic_source("bitwise-not", source);
    assert_eq!(stdout, "-6\n");
}

#[test]
fn bitwise_shift_left_produces_correct_result() {
    let source = "defmodule Demo do\n  def run() do\n    1 <<< 4\n  end\nend\n";
    let stdout = run_tonic_source("bitwise-shift-left", source);
    assert_eq!(stdout, "16\n");
}

#[test]
fn bitwise_shift_right_produces_correct_result() {
    let source = "defmodule Demo do\n  def run() do\n    16 >>> 2\n  end\nend\n";
    let stdout = run_tonic_source("bitwise-shift-right", source);
    assert_eq!(stdout, "4\n");
}

#[test]
fn bitwise_operators_fixture_produces_expected_output() {
    let source = "defmodule Demo do\n  def run() do\n    {5 &&& 3, {5 ||| 3, {5 ^^^ 6, {~~~5, {1 <<< 4, 16 >>> 2}}}}}\n  end\nend\n";
    let stdout = run_tonic_source("bitwise-operators-fixture", source);
    assert_eq!(stdout, "{1, {7, {3, {-6, {16, 4}}}}}\n");
}

// ============================================================
// Feature 2e: Stepped ranges (first..last//step)
// ============================================================

#[test]
fn stepped_range_for_comprehension_produces_expected_list() {
    let source = "defmodule Demo do\n  def run() do\n    for x <- 1..10//3 do\n      x\n    end\n  end\nend\n";
    let stdout = run_tonic_source("stepped-range-for", source);
    assert_eq!(stdout, "[1, 4, 7, 10]\n");
}

#[test]
fn stepped_range_step_two_skips_evens() {
    let source = "defmodule Demo do\n  def run() do\n    for x <- 1..9//2 do\n      x\n    end\n  end\nend\n";
    let stdout = run_tonic_source("stepped-range-step-2", source);
    assert_eq!(stdout, "[1, 3, 5, 7, 9]\n");
}

#[test]
fn stepped_range_fixture_produces_expected_output() {
    let source = "defmodule Demo do\n  def run() do\n    for x <- 1..10//3 do\n      x\n    end\n  end\nend\n";
    let stdout = run_tonic_source("stepped-range-fixture", source);
    assert_eq!(stdout, "[1, 4, 7, 10]\n");
}
