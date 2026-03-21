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
        stderr.contains("expected / in function capture"),
        "unexpected parser diagnostic: {stderr}"
    );
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
