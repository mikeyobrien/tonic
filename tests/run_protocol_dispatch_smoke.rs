use std::fs;
mod common;

#[test]
fn run_dispatches_protocol_call_for_tuple_and_map_values() {
    let fixture_root = common::unique_fixture_root("run-protocol-dispatch-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_protocol_dispatch.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(protocol_dispatch(tuple(1, 2)), protocol_dispatch(map(3, 4)))\n  end\nend\n",
    )
    .expect("fixture setup should write protocol dispatch source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_protocol_dispatch.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "{1, 2}\n");
}
