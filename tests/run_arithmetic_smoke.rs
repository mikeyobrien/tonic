use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_arithmetic_entrypoint_and_prints_result() {
    let fixture_root = unique_fixture_root("run-arithmetic-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_smoke.tn"),
        "defmodule Demo do\n  def run() do\n    1 + 2\n  end\nend\n",
    )
    .expect("fixture setup should write run smoke source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_smoke.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "3\n");
}

#[test]
fn run_executes_subtraction() {
    let fixture_root = unique_fixture_root("run-subtraction");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("sub.tn"),
        "defmodule Demo do\n  def run() do\n    5 - 3\n  end\nend\n",
    )
    .expect("fixture setup should write subtraction source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/sub.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "2\n"
    );
}

#[test]
fn run_executes_multiplication() {
    let fixture_root = unique_fixture_root("run-multiplication");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("mul.tn"),
        "defmodule Demo do\n  def run() do\n    3 * 4\n  end\nend\n",
    )
    .expect("fixture setup should write multiplication source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/mul.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "12\n"
    );
}

#[test]
fn run_executes_division() {
    let fixture_root = unique_fixture_root("run-division");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("div.tn"),
        "defmodule Demo do\n  def run() do\n    10 / 2\n  end\nend\n",
    )
    .expect("fixture setup should write division source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/div.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "5\n"
    );
}

#[test]
fn run_precedence_mul_before_add_yields_fourteen() {
    let fixture_root = unique_fixture_root("run-precedence-mul-add");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("precedence.tn"),
        "defmodule Demo do\n  def run() do\n    2 + 3 * 4\n  end\nend\n",
    )
    .expect("fixture setup should write precedence source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/precedence.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "14\n"
    );
}

#[test]
fn run_comparison_gt_returns_bool() {
    let fixture_root = unique_fixture_root("run-comparison-gt");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("cmp_gt.tn"),
        "defmodule Demo do\n  def run() do\n    3 > 2\n  end\nend\n",
    )
    .expect("fixture setup should write comparison source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/cmp_gt.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "true\n"
    );
}

#[test]
fn run_comparison_eq_returns_false_for_unequal_ints() {
    let fixture_root = unique_fixture_root("run-comparison-eq-false");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("cmp_eq.tn"),
        "defmodule Demo do\n  def run() do\n    3 == 4\n  end\nend\n",
    )
    .expect("fixture setup should write equality source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/cmp_eq.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "false\n"
    );
}

#[test]
fn run_executes_remaining_comparison_operators() {
    let fixture_root = unique_fixture_root("run-comparison-remaining");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("cmp_remaining.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(tuple(3 != 4, 2 < 3), tuple(3 <= 3, 4 >= 5))\n  end\nend\n",
    )
    .expect("fixture setup should write comparison source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/cmp_remaining.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "{{true, true}, {true, false}}\n"
    );
}

#[test]
fn run_check_rejects_non_int_comparison_right_operand_with_type_mismatch() {
    let fixture_root = unique_fixture_root("run-check-bool-comparison");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("bool_cmp.tn"),
        "defmodule Demo do\n  def run() do\n    1 < false\n  end\nend\n",
    )
    .expect("fixture setup should write comparison source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/bool_cmp.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check to fail for bool in comparison context, got status {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E2001] type mismatch: expected int, found bool at offset 41\n"
    );
}

#[test]
fn run_division_by_zero_produces_runtime_error() {
    let fixture_root = unique_fixture_root("run-div-by-zero");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("div_zero.tn"),
        "defmodule Demo do\n  def run() do\n    1 / 0\n  end\nend\n",
    )
    .expect("fixture setup should write division by zero source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/div_zero.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run to fail on division by zero, got status {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("division by zero"),
        "expected 'division by zero' in stderr, got: {stderr}"
    );
}

#[test]
fn run_check_rejects_non_int_left_operand_with_type_mismatch() {
    let fixture_root = unique_fixture_root("run-check-bool-arithmetic");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("bool_add.tn"),
        "defmodule Demo do\n  def run() do\n    true + 1\n  end\nend\n",
    )
    .expect("fixture setup should write bool arithmetic source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/bool_add.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check to fail for bool in arithmetic context, got status {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E2001] type mismatch: expected int, found bool at offset 37\n"
    );
}

#[test]
fn run_executes_unary_plus_and_minus() {
    let fixture_root = unique_fixture_root("run-unary-plus-minus");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unary_plus_minus.tn"),
        "defmodule Demo do\n  def run() do\n    -5 + +2\n  end\nend\n",
    )
    .expect("fixture setup should write unary arithmetic source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/unary_plus_minus.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "-3\n"
    );
}

#[test]
fn run_check_rejects_unary_minus_on_non_int_operand() {
    let fixture_root = unique_fixture_root("run-check-unary-minus-bool");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unary_minus_bool.tn"),
        "defmodule Demo do\n  def run() do\n    -true\n  end\nend\n",
    )
    .expect("fixture setup should write unary bool source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unary_minus_bool.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check to fail for unary minus on bool, got status {:?}",
        output.status.code()
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E2001] type mismatch: expected int, found bool at offset 38\n"
    );
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}

#[test]
fn run_executes_ergonomic_case_and_patterns() {
    let fixture_root = unique_fixture_root("run-ergonomics-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_ergonomics.tn"),
        "defmodule Demo do\n  def get_status() do\n    tuple(:ok, 200)\n  end\n\n  def route(status) do\n    case status do\n      {:ok, 200} -> 42\n      _ -> 0\n    end\n  end\n\n  def run() do\n    route(get_status())\n  end\nend\n",
    )
    .expect("fixture setup should write ergonomics source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_ergonomics.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "42\n");
}
