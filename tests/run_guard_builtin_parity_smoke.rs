use std::fs;
mod common;

#[test]
fn run_executes_guard_builtin_function_and_case_guards() {
    let fixture_root = common::unique_fixture_root("run-guard-builtin-parity");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_guard_builtin_parity.tn"),
        "defmodule Demo do\n  def guarded(value) when is_integer(value) do\n    :integer\n  end\n\n  def guarded(value) when is_binary(value) do\n    :binary\n  end\n\n  def guarded(value) when is_list(value) do\n    :list\n  end\n\n  def guarded(value) when is_map(value) do\n    :map\n  end\n\n  def guarded(value) when is_nil(value) do\n    :nil\n  end\n\n  def guarded(_value) do\n    :other\n  end\n\n  def classify(value) do\n    case value do\n      _ when is_float(value) -> :float\n      _ when is_number(value) -> :number\n      _ when is_atom(value) -> :atom\n      _ when is_tuple(value) -> :tuple\n      _ -> :fallback\n    end\n  end\n\n  def run() do\n    {\n      {guarded(1), guarded(\"hi\")},\n      {\n        {guarded(list(1)), {guarded(map(:ok, 1)), {guarded(nil), guarded(true)}}},\n        {classify(1.5), {classify(1), {classify(:ok), {classify(tuple(1, 2)), classify(list(1))}}}}\n      }\n    }\n  end\nend\n",
    )
    .expect("fixture setup should write guard builtin parity source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_guard_builtin_parity.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout,
        "{{:integer, :binary}, {{:list, {:map, {:nil, :other}}}, {:float, {:number, {:atom, {:tuple, :fallback}}}}}}\n"
    );
}

#[test]
fn check_rejects_guard_builtin_calls_outside_guards() {
    let fixture_root = common::unique_fixture_root("check-guard-builtin-outside-when");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("check_guard_builtin_outside_when.tn"),
        "defmodule Demo do\n  def run() do\n    is_integer(1)\n  end\nend\n",
    )
    .expect("fixture setup should write guard builtin misuse source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/check_guard_builtin_outside_when.tn"])
        .output()
        .expect("check command should run");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E1015] guard builtin 'is_integer/1' is only allowed in guard expressions (when) in Demo.run\n"
    );
}
