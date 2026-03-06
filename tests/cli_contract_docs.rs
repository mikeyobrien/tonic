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

    let out_dir = fixture_root.join("docs/api");

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

    // stdout should contain the summary line
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(
        stdout.contains("Generated docs"),
        "stdout should have summary, got:\n{stdout}"
    );
    assert!(
        stdout.contains("module"),
        "stdout summary should mention 'module', got:\n{stdout}"
    );

    // Module markdown file should be written
    let module_file = out_dir.join("mymodule.md");
    assert!(
        module_file.exists(),
        "mymodule.md should be created at {}",
        module_file.display()
    );

    let content = std::fs::read_to_string(&module_file).unwrap();

    // Should contain module name
    assert!(
        content.contains("MyModule"),
        "module file should contain module name, got:\n{content}"
    );

    // Should contain moduledoc
    assert!(
        content.contains("A sample module for documentation"),
        "module file should contain moduledoc, got:\n{content}"
    );

    // Should contain function names
    assert!(
        content.contains("add"),
        "module file should contain function name 'add', got:\n{content}"
    );

    // Should contain doc strings
    assert!(
        content.contains("Adds two numbers"),
        "module file should contain doc string, got:\n{content}"
    );

    // Index file should be written
    let index_file = out_dir.join("index.md");
    assert!(index_file.exists(), "index.md should be created");
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

    let out_dir = fixture_root.join("docs/api");

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
        stdout.contains("Generated docs"),
        "stdout should have summary, got:\n{stdout}"
    );

    let module_file = out_dir.join("bare.md");
    assert!(
        module_file.exists(),
        "bare.md should be created at {}",
        module_file.display()
    );

    let content = std::fs::read_to_string(&module_file).unwrap();
    assert!(
        content.contains("Bare"),
        "module file should contain module name, got:\n{content}"
    );
}

#[test]
fn docs_errors_without_path() {
    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .args(["docs"])
        .output()
        .expect("tonic docs should run");

    assert!(!output.status.success(), "docs without path should fail");
}

#[test]
fn docs_respects_output_flag() {
    let fixture_root = common::unique_fixture_root("cli-docs-output");
    let examples_dir = fixture_root.join("examples");
    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("mypkg.tn"),
        "defmodule Pkg do\n  def run() do 1 end\nend\n",
    )
    .unwrap();

    let custom_out = fixture_root.join("custom-docs");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args([
            "docs",
            "examples/mypkg.tn",
            "--output",
            custom_out.to_str().unwrap(),
        ])
        .output()
        .expect("tonic docs should run");

    assert!(
        output.status.success(),
        "docs with --output should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let module_file = custom_out.join("pkg.md");
    assert!(
        module_file.exists(),
        "pkg.md should be created in custom dir at {}",
        module_file.display()
    );
}

#[test]
fn docs_generates_stdlib_subdirectory() {
    let fixture_root = common::unique_fixture_root("cli-docs-stdlib");
    let examples_dir = fixture_root.join("examples");
    std::fs::create_dir_all(&examples_dir).unwrap();
    std::fs::write(
        examples_dir.join("main.tn"),
        "defmodule Main do\n  def run() do 1 end\nend\n",
    )
    .unwrap();

    let out_dir = fixture_root.join("docs/api");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["docs", "examples/main.tn"])
        .output()
        .expect("tonic docs should run");

    assert!(
        output.status.success(),
        "docs should exit 0, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // stdlib subdir should exist with at least System and Enum
    let stdlib_dir = out_dir.join("stdlib");
    assert!(
        stdlib_dir.exists(),
        "stdlib/ subdirectory should be created"
    );

    let system_file = stdlib_dir.join("system.md");
    assert!(
        system_file.exists(),
        "system.md should exist in stdlib/, dir contents: {:?}",
        std::fs::read_dir(&stdlib_dir)
            .map(|d| d.map(|e| e.map(|e| e.file_name())).collect::<Vec<_>>())
            .unwrap_or_default()
    );
}
