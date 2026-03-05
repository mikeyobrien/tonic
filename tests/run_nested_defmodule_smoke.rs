use std::fs;

mod common;

#[test]
fn run_nested_defmodule_dotted_name() {
    let fixture_root = common::unique_fixture_root("run-nested-defmodule-dotted");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup");
    fs::write(
        examples_dir.join("nested_defmodule.tn"),
        "defmodule Foo do\n  defmodule Bar do\n    def greet() do\n      :hello\n    end\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Foo.Bar.greet()\n  end\nend\n",
    )
    .expect("write fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/nested_defmodule.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, ":hello\n");
}

#[test]
fn run_nested_defmodule_function_in_parent() {
    let fixture_root = common::unique_fixture_root("run-nested-defmodule-parent-fn");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup");
    fs::write(
        examples_dir.join("nested_parent_fn.tn"),
        "defmodule Outer do\n  defmodule Inner do\n    def value() do\n      10\n    end\n  end\n\n  def double() do\n    Outer.Inner.value() * 2\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Outer.double()\n  end\nend\n",
    )
    .expect("write fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/nested_parent_fn.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "20\n");
}
