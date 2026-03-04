mod common;

use std::process::Command;

#[test]
fn docs_produces_output_for_documented_module() {
    let fixture_root = common::unique_fixture_root("cli-docs");
    let examples_dir = fixture_root.join("examples");
    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("documented.tn"),
        r#"defmodule MyModule do
  @moduledoc "A sample module for documentation."
  @doc "Adds two numbers."
  def add(a, b) do
    a + b
  end

  @doc "Multiplies two numbers."
  def multiply(a, b) do
    a * b
  end

  def run() do
    add(1, 2)
  end
end
"#,
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["docs", "examples/documented.tn"])
        .output()
        .expect("tonic docs should run");

    assert!(
        output.status.success(),
        "docs should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    
    // Should contain module name
    assert!(
        stdout.contains("MyModule"),
        "output should contain module name, got:\n{stdout}"
    );
    
    // Should contain moduledoc
    assert!(
        stdout.contains("A sample module for documentation"),
        "output should contain moduledoc, got:\n{stdout}"
    );
    
    // Should contain function names
    assert!(
        stdout.contains("add"),
        "output should contain function name 'add', got:\n{stdout}"
    );
    
    // Should contain doc strings
    assert!(
        stdout.contains("Adds two numbers"),
        "output should contain doc string, got:\n{stdout}"
    );
}

#[test]
fn docs_succeeds_for_undocumented_module() {
    let fixture_root = common::unique_fixture_root("cli-docs-undoc");
    let examples_dir = fixture_root.join("examples");
    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("undocumented.tn"),
        "defmodule Bare do\n  def run() do\n    42\n  end\nend\n",
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["docs", "examples/undocumented.tn"])
        .output()
        .expect("tonic docs should run");

    assert!(
        output.status.success(),
        "docs should exit 0 even for undocumented module, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Bare"),
        "output should contain module name, got:\n{stdout}"
    );
}

#[test]
fn docs_errors_without_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .args(["docs"])
        .output()
        .expect("tonic docs should run");

    assert!(
        !output.status.success(),
        "docs without path should fail"
    );
}
