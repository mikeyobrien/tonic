use std::fs;
mod common;

#[test]
fn run_dispatches_protocol_calls_for_tuple_map_and_struct_values() {
    let fixture_root = common::unique_fixture_root("run-protocol-defimpl-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_protocol_defimpl.tn"),
        "defmodule User do\n  defstruct age: 0\nend\n\ndefmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      2\n    end\n  end\n\n  defimpl Size, for: Map do\n    def size(_value) do\n      1\n    end\n  end\n\n  defimpl Size, for: User do\n    def size(user) do\n      user.age\n    end\n  end\n\n  def run() do\n    tuple(Size.size(tuple(1, 2)), tuple(Size.size(%{a: 1}), Size.size(%User{age: 9})))\n  end\nend\n",
    )
    .expect("fixture setup should write protocol dispatch source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_protocol_defimpl.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "{2, {1, 9}}\n");
}

#[test]
fn run_reports_missing_protocol_impl_for_map_value() {
    let fixture_root = common::unique_fixture_root("run-protocol-defimpl-missing-map");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_protocol_defimpl_missing_map.tn"),
        "defmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      2\n    end\n  end\n\n  def run() do\n    Size.size(%{a: 1})\n  end\nend\n",
    )
    .expect("fixture setup should write missing map impl source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_protocol_defimpl_missing_map.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run command to fail for missing protocol implementation"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("protocol Size.size has no implementation for map"),
        "unexpected runtime diagnostic: {stderr}"
    );
}
