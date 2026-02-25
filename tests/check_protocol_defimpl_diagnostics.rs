use std::fs;
mod common;

#[test]
fn check_reports_unknown_protocol_for_defimpl() {
    let fixture_root = common::unique_fixture_root("check-unknown-protocol-defimpl");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("unknown_protocol_defimpl.tn"),
        "defmodule Demo do\n  defimpl Missing, for: Tuple do\n    def size(_value) do\n      1\n    end\n  end\n\n  def run() do\n    0\n  end\nend\n",
    )
    .expect("fixture setup should write unknown protocol source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/unknown_protocol_defimpl.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for unknown defimpl protocol"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("[E1008] unknown protocol 'Missing' for defimpl target 'Tuple'"),
        "unexpected resolver diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_duplicate_protocol_impl_target() {
    let fixture_root = common::unique_fixture_root("check-duplicate-defimpl-target");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("duplicate_protocol_impl_target.tn"),
        "defmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      1\n    end\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      2\n    end\n  end\n\n  def run() do\n    0\n  end\nend\n",
    )
    .expect("fixture setup should write duplicate defimpl source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/duplicate_protocol_impl_target.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for duplicate defimpl target"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("[E1009] duplicate defimpl for protocol 'Size' and target 'Tuple'"),
        "unexpected resolver diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_missing_protocol_function_in_impl() {
    let fixture_root = common::unique_fixture_root("check-missing-protocol-function-in-impl");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("missing_protocol_function_in_impl.tn"),
        "defmodule Demo do\n  defprotocol Size do\n    def size(value)\n    def label(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      1\n    end\n  end\n\n  def run() do\n    0\n  end\nend\n",
    )
    .expect("fixture setup should write missing protocol function source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/missing_protocol_function_in_impl.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for incomplete protocol implementation"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(
            "[E1010] invalid defimpl for protocol 'Size' target 'Tuple': label/1 is missing"
        ),
        "unexpected resolver diagnostic: {stderr}"
    );
}
