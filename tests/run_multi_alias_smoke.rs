use std::fs;

mod common;

#[test]
fn run_multi_alias_expands_to_individual_aliases() {
    let fixture_root = common::unique_fixture_root("run-multi-alias-expand");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup");
    fs::write(
        examples_dir.join("multi_alias.tn"),
        "defmodule Foo.Bar do\n  def value() do\n    1\n  end\nend\n\ndefmodule Foo.Baz do\n  def value() do\n    2\n  end\nend\n\ndefmodule Demo do\n  alias Foo.{Bar, Baz}\n\n  def run() do\n    Bar.value() + Baz.value()\n  end\nend\n",
    )
    .expect("write fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/multi_alias.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "3\n");
}

#[test]
fn run_multi_alias_single_module() {
    let fixture_root = common::unique_fixture_root("run-multi-alias-single");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup");
    fs::write(
        examples_dir.join("multi_alias_single.tn"),
        "defmodule Foo.Bar do\n  def value() do\n    42\n  end\nend\n\ndefmodule Demo do\n  alias Foo.{Bar}\n\n  def run() do\n    Bar.value()\n  end\nend\n",
    )
    .expect("write fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/multi_alias_single.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "42\n");
}
