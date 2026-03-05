use std::fs;

mod common;

#[test]
fn run_module_variable_returns_module_name() {
    let fixture_root = common::unique_fixture_root("run-module-special-vars");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup");
    fs::write(
        examples_dir.join("module_var.tn"),
        "defmodule MyApp.Greeter do\n  def name() do\n    __MODULE__\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    MyApp.Greeter.name()\n  end\nend\n",
    )
    .expect("write fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/module_var.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    // __MODULE__ resolves to the module name atom (printed with leading colon)
    assert_eq!(stdout, ":MyApp.Greeter\n");
}

#[test]
fn run_module_variable_in_nested_module() {
    let fixture_root = common::unique_fixture_root("run-module-var-nested");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup");
    fs::write(
        examples_dir.join("nested_module_var.tn"),
        "defmodule Outer do\n  defmodule Inner do\n    def name() do\n      __MODULE__\n    end\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Outer.Inner.name()\n  end\nend\n",
    )
    .expect("write fixture");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/nested_module_var.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, ":Outer.Inner\n");
}
