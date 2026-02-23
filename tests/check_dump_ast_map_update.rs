use std::fs;
mod common;

#[test]
fn test_dump() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-map-update");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    // Use two separate functions so each body is a single expression;
    // the parser does not yet support multi-statement do-blocks.
    fs::write(
        examples_dir.join("map_update.tn"),
        "defmodule Demo do\n  def run() do\n    update(%{a: 1})\n  end\n  def update(b) do\n    %{b | a: 2}\n  end\nend\n",
    )
    .expect("fixture setup should write parser expression source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/map_update.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
}
