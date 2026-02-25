use std::fs;

mod common;

#[test]
fn run_executes_import_only_semantics() {
    let fixture_root = common::unique_fixture_root("run-import-only-semantics");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("import_only_semantics.tn"),
        "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\n\n  def unsafe(value) do\n    value - 1\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [add: 2]\n\n  def run() do\n    add(20, 22)\n  end\nend\n",
    )
    .expect("fixture setup should write import only source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/import_only_semantics.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run invocation to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "42\n");
}

#[test]
fn run_executes_import_except_semantics() {
    let fixture_root = common::unique_fixture_root("run-import-except-semantics");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("import_except_semantics.tn"),
        "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\n\n  def unsafe(value) do\n    value - 1\n  end\nend\n\ndefmodule Demo do\n  import Math, except: [unsafe: 1]\n\n  def run() do\n    add(1, 2)\n  end\nend\n",
    )
    .expect("fixture setup should write import except source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/import_except_semantics.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run invocation to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "3\n");
}

#[test]
fn check_reports_malformed_import_option_payload() {
    let fixture_root = common::unique_fixture_root("check-malformed-import-option-payload");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("malformed_import_option.tn"),
        "defmodule Demo do\n  import Math, only: [helper]\n\n  def run() do\n    helper(1)\n  end\nend\n",
    )
    .expect("fixture setup should write malformed import option source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/malformed_import_option.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check to fail for malformed import option payload"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("invalid import only option; expected only: [name: arity, ...]"),
        "unexpected malformed import diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_import_filter_exclusion_and_ambiguity() {
    let fixture_root = common::unique_fixture_root("check-import-filter-diagnostics");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("import_filter_exclusion.tn"),
        "defmodule Math do\n  def helper(value) do\n    value\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [other: 1]\n\n  def run() do\n    helper(1)\n  end\nend\n",
    )
    .expect("fixture setup should write exclusion source file");
    fs::write(
        examples_dir.join("import_filter_ambiguous.tn"),
        "defmodule Math do\n  def helper(value) do\n    value\n  end\nend\n\ndefmodule Helpers do\n  def helper(value) do\n    value + 1\n  end\nend\n\ndefmodule Demo do\n  import Math, except: [other: 1]\n  import Helpers, only: [helper: 1]\n\n  def run() do\n    helper(1)\n  end\nend\n",
    )
    .expect("fixture setup should write ambiguity source file");

    let exclusion = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/import_filter_exclusion.tn"])
        .output()
        .expect("check command should run for exclusion fixture");

    assert!(
        !exclusion.status.success(),
        "expected check to fail for excluded imported call"
    );

    let exclusion_stderr = String::from_utf8(exclusion.stderr).expect("stderr should be utf8");
    assert!(
        exclusion_stderr.contains(
            "[E1013] import filters exclude call 'helper/1' in Demo; imported modules with this symbol: Math"
        ),
        "unexpected exclusion diagnostic: {exclusion_stderr}"
    );

    let ambiguous = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/import_filter_ambiguous.tn"])
        .output()
        .expect("check command should run for ambiguity fixture");

    assert!(
        !ambiguous.status.success(),
        "expected check to fail for ambiguous imported call"
    );

    let ambiguous_stderr = String::from_utf8(ambiguous.stderr).expect("stderr should be utf8");
    assert!(
        ambiguous_stderr
            .contains("[E1014] ambiguous imported call 'helper/1' in Demo; matches: Helpers, Math"),
        "unexpected ambiguity diagnostic: {ambiguous_stderr}"
    );
}
