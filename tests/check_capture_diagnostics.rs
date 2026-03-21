use std::fs;
mod common;

#[test]
fn check_reports_invalid_named_capture_syntax() {
    let fixture_root = common::unique_fixture_root("check-invalid-named-capture-syntax");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("invalid_named_capture.tn"),
        "defmodule Demo do\n  def run() do\n    &Math.add\n  end\nend\n",
    )
    .expect("fixture setup should write invalid capture source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/invalid_named_capture.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for malformed function capture syntax"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0009] missing '/arity' in named function capture `&Math.add`. hint: write `&Math.add/arity`, for example `&Math.add/2` if the function takes two arguments"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(stderr.contains(" --> examples/invalid_named_capture.tn:3:5"));
    assert!(stderr.contains("3 |     &Math.add"));
}

#[test]
fn check_reports_empty_capture_expression_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-empty-capture-expression");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("empty_capture_expression.tn"),
        "defmodule Demo do\n  def run() do\n    &()\n  end\nend\n",
    )
    .expect("fixture setup should write empty capture source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/empty_capture_expression.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for empty capture expressions"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0009] empty capture expression `&()`. hint: wrap an expression that uses placeholders, for example `&(&1 + 1)` or `&(expr_with_&1)`"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(stderr.contains(" --> examples/empty_capture_expression.tn:3:5"));
    assert!(stderr.contains("3 |     &()"));
}

#[test]
fn check_reports_invalid_capture_placeholder_zero_with_fix_hint() {
    let fixture_root = common::unique_fixture_root("check-invalid-capture-placeholder-zero");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("invalid_capture_placeholder_zero.tn"),
        "defmodule Demo do\n  def run() do\n    (&0)\n  end\nend\n",
    )
    .expect("fixture setup should write invalid placeholder source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/invalid_capture_placeholder_zero.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for invalid &0 placeholders"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E0009] invalid capture placeholder `&0`. hint: capture placeholders start at `&1`; replace `&0` with `&1` or another positive index"
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(stderr.contains(" --> examples/invalid_capture_placeholder_zero.tn:3:6"));
    assert!(stderr.contains("3 |     (&0)"));
}

#[test]
fn check_reports_unknown_named_capture_target() {
    let fixture_root = common::unique_fixture_root("check-unknown-named-capture-target");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unknown_named_capture.tn"),
        "defmodule Demo do\n  def run() do\n    (&Missing.add/2).(1, 2)\n  end\nend\n",
    )
    .expect("fixture setup should write unknown capture target source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unknown_named_capture.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for unknown named capture target"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: [E1001] undefined symbol 'Missing.add' in Demo.run"),
        "unexpected resolver diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_named_capture_target_typo_with_repair_hint() {
    let fixture_root = common::unique_fixture_root("check-named-capture-target-typo");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("named_capture_target_typo.tn"),
        "defmodule Math do\n  def add(left, right) do\n    left + right\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    (&Math.ad/2).(1, 2)\n  end\nend\n",
    )
    .expect("fixture setup should write capture target typo source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/named_capture_target_typo.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for named capture target typo"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E1001] undefined symbol 'Math.ad' in Demo.run; did you mean `Math.add/2`?. Available Math functions: add"
        ),
        "unexpected named capture typo diagnostic: {stderr}"
    );
    assert!(stderr.contains("--> examples/named_capture_target_typo.tn:9:6"));
    assert!(stderr.contains("9 |     (&Math.ad/2).(1, 2)"));
}

#[test]
fn check_reports_capture_arity_mismatch_against_target_signature() {
    let fixture_root = common::unique_fixture_root("check-named-capture-arity-mismatch");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("named_capture_arity_mismatch.tn"),
        "defmodule Math do\n  def add(left, right) do\n    left + right\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    (&Math.add/3).(1, 2, 3)\n  end\nend\n",
    )
    .expect("fixture setup should write named capture arity mismatch source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/named_capture_arity_mismatch.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for named capture arity mismatch"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "error: [E2002] arity mismatch for Math.add: expected 2 args, found 3; hint: call `Math.add/2`"
        ),
        "unexpected typing diagnostic: {stderr}"
    );
    assert!(stderr.contains("--> examples/named_capture_arity_mismatch.tn:9:6"));
    assert!(stderr.contains("9 |     (&Math.add/3).(1, 2, 3)"));
}
