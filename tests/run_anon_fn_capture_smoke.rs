use std::fs;
mod common;

#[test]
fn run_executes_anonymous_function_with_lexical_capture() {
    let fixture_root = common::unique_fixture_root("run-anon-fn-lexical-capture");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_anon_fn_capture.tn"),
        "defmodule Demo do\n  def make_adder(base) do\n    fn value -> value + base end\n  end\n\n  def run() do\n    make_adder(4).(3)\n  end\nend\n",
    )
    .expect("fixture setup should write anonymous function source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_anon_fn_capture.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "7\n"
    );
}

#[test]
fn run_executes_capture_shorthand_with_placeholder_expansion() {
    let fixture_root = common::unique_fixture_root("run-capture-shorthand");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_capture_shorthand.tn"),
        "defmodule Demo do\n  def run() do\n    (&(&1 + 1)).(41)\n  end\nend\n",
    )
    .expect("fixture setup should write capture shorthand source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_capture_shorthand.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "42\n"
    );
}

#[test]
fn run_executes_named_function_capture_with_explicit_arity() {
    let fixture_root = common::unique_fixture_root("run-named-function-capture");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_named_capture.tn"),
        "defmodule Math do\n  def add(left, right) do\n    left + right\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    (&Math.add/2).(20, 22)\n  end\nend\n",
    )
    .expect("fixture setup should write named capture source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_named_capture.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "42\n"
    );
}

#[test]
fn run_executes_multi_clause_anonymous_function_with_guards_in_order() {
    let fixture_root = common::unique_fixture_root("run-multi-clause-anon-function");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_multi_clause_anon_fn.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(\n      (fn\n        {:ok, value} when is_integer(value) -> value\n        {:ok, _} -> -1\n        _ -> 0\n      end).({:ok, 7}),\n      tuple(\n        (fn\n          {:ok, value} when is_integer(value) -> value\n          {:ok, _} -> -1\n          _ -> 0\n        end).({:ok, \"nope\"}),\n        (fn\n          {:ok, value} when is_integer(value) -> value\n          {:ok, _} -> -1\n          _ -> 0\n        end).(:error)\n      )\n    )\n  end\nend\n",
    )
    .expect("fixture setup should write multi-clause anonymous function source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_multi_clause_anon_fn.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "{7, {-1, 0}}\n"
    );
}
