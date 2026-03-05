use std::fs;
mod common;

// ── div operator ─────────────────────────────────────────────────────────────

#[test]
fn run_div_operator_basic() {
    let fixture_root = common::unique_fixture_root("run-div-operator-basic");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("div_basic.tn"),
        "defmodule Demo do\n  def run() do\n    10 div 3\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/div_basic.tn"])
        .output()
        .expect("execute");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "3\n"
    );
}

#[test]
fn run_div_operator_truncates_toward_zero_negative() {
    let fixture_root = common::unique_fixture_root("run-div-neg-truncate");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("div_neg.tn"),
        "defmodule Demo do\n  def run() do\n    -7 div 2\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/div_neg.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    // -7 div 2 = -3 (truncation toward zero)
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "-3\n"
    );
}

#[test]
fn run_div_operator_by_zero_errors() {
    let fixture_root = common::unique_fixture_root("run-div-by-zero-op");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("div_zero.tn"),
        "defmodule Demo do\n  def run() do\n    5 div 0\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/div_zero.tn"])
        .output()
        .expect("execute");

    assert!(!output.status.success(), "expected failure on div by zero");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("division by zero"),
        "expected 'division by zero' in stderr, got: {stderr}"
    );
}

// ── rem operator ─────────────────────────────────────────────────────────────

#[test]
fn run_rem_operator_basic() {
    let fixture_root = common::unique_fixture_root("run-rem-operator-basic");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("rem_basic.tn"),
        "defmodule Demo do\n  def run() do\n    10 rem 3\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/rem_basic.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "1\n"
    );
}

#[test]
fn run_rem_operator_sign_follows_dividend() {
    let fixture_root = common::unique_fixture_root("run-rem-sign-dividend");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("rem_sign.tn"),
        "defmodule Demo do\n  def run() do\n    -7 rem 2\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/rem_sign.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    // -7 rem 2 = -1 (sign follows dividend)
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "-1\n"
    );
}

#[test]
fn run_rem_by_zero_errors() {
    let fixture_root = common::unique_fixture_root("run-rem-by-zero");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("rem_zero.tn"),
        "defmodule Demo do\n  def run() do\n    5 rem 0\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/rem_zero.tn"])
        .output()
        .expect("execute");

    assert!(!output.status.success(), "expected failure on rem by zero");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("remainder by zero") || stderr.contains("division by zero"),
        "expected 'remainder by zero' in stderr, got: {stderr}"
    );
}

// ── div/rem precedence ────────────────────────────────────────────────────────

#[test]
fn run_div_has_same_precedence_as_mul() {
    let fixture_root = common::unique_fixture_root("run-div-precedence");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    // 2 + 10 div 3 should be 2 + 3 = 5 (div binds tighter than +)
    fs::write(
        examples_dir.join("div_prec.tn"),
        "defmodule Demo do\n  def run() do\n    2 + 10 div 3\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/div_prec.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "5\n"
    );
}

// ── not in operator ───────────────────────────────────────────────────────────

#[test]
fn run_not_in_returns_true_for_absent_element() {
    let fixture_root = common::unique_fixture_root("run-not-in-absent");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("not_in.tn"),
        "defmodule Demo do\n  def run() do\n    5 not in [1, 2, 3]\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/not_in.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "true\n"
    );
}

#[test]
fn run_not_in_returns_false_for_present_element() {
    let fixture_root = common::unique_fixture_root("run-not-in-present");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("not_in_present.tn"),
        "defmodule Demo do\n  def run() do\n    2 not in [1, 2, 3]\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/not_in_present.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "false\n"
    );
}

#[test]
fn run_not_in_works_in_range() {
    let fixture_root = common::unique_fixture_root("run-not-in-range");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("not_in_range.tn"),
        "defmodule Demo do\n  def run() do\n    10 not in 1..5\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/not_in_range.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "true\n"
    );
}

// ── stepped ranges ────────────────────────────────────────────────────────────

#[test]
fn run_stepped_range_for_loop() {
    let fixture_root = common::unique_fixture_root("run-stepped-range-for");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("stepped_range.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- 1..10//3 do\n      x\n    end\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/stepped_range.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "[1, 4, 7, 10]\n"
    );
}

#[test]
fn run_stepped_range_even_numbers() {
    let fixture_root = common::unique_fixture_root("run-stepped-range-even");
    let examples_dir = fixture_root.join("examples");
    fs::create_dir_all(&examples_dir).expect("setup");
    fs::write(
        examples_dir.join("stepped_even.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- 0..8//2 do\n      x\n    end\n  end\nend\n",
    )
    .expect("write");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/stepped_even.tn"])
        .output()
        .expect("execute");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).expect("utf8"),
        "[0, 2, 4, 6, 8]\n"
    );
}
