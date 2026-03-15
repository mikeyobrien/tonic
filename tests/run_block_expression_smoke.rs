use std::fs;
mod common;

fn run_source(test_name: &str, source: &str) -> (String, String, bool) {
    let root = common::unique_fixture_root(test_name);
    fs::create_dir_all(&root).expect("create fixture dir");
    let file = root.join("test.tn");
    fs::write(&file, source).expect("write source");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&root)
        .args(["run", "test.tn"])
        .output()
        .expect("run tonic");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (stdout, stderr, output.status.success())
}

#[test]
fn block_sequential_assignments_in_def_body() {
    let (stdout, stderr, ok) = run_source(
        "block-seq-assign",
        "defmodule Demo do\n  def run() do\n    x = 1\n    y = 2\n    x + y\n  end\nend\n",
    );
    assert!(ok, "expected success, stderr: {stderr}");
    assert_eq!(stdout.trim(), "3");
}

#[test]
fn block_sequential_statements_with_stdlib_single_file() {
    let (stdout, stderr, ok) = run_source(
        "block-stdlib-single",
        "defmodule Demo do\n  def run() do\n    items = [1, 2, 3]\n    Enum.count(items)\n  end\nend\n",
    );
    assert!(ok, "expected success, stderr: {stderr}");
    assert_eq!(stdout.trim(), "3");
}

#[test]
fn block_in_if_then_and_else() {
    let (stdout, stderr, ok) = run_source(
        "block-if-else",
        "defmodule Demo do\n  def run() do\n    x = 10\n    if x > 5 do\n      a = x * 2\n      a + 1\n    else\n      b = x - 1\n      b\n    end\n  end\nend\n",
    );
    assert!(ok, "expected success, stderr: {stderr}");
    assert_eq!(stdout.trim(), "21");
}

#[test]
fn block_in_for_body() {
    let (stdout, stderr, ok) = run_source(
        "block-for-body",
        "defmodule Demo do\n  def run() do\n    for x <- [1, 2, 3] do\n      doubled = x * 2\n      doubled\n    end\n  end\nend\n",
    );
    assert!(ok, "expected success, stderr: {stderr}");
    assert_eq!(stdout.trim(), "[2, 4, 6]");
}
