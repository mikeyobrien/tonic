use std::fs;
mod common;

#[test]
fn compile_dump_mir_emits_json_instead_of_artifact_path_output() {
    let temp_dir = common::unique_temp_dir("compile-dump-mir");
    let source_path = temp_dir.join("dump_mir.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "dump_mir.tn", "--dump-mir"])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected compile --dump-mir success, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let trimmed = stdout.trim();
    let parsed: serde_json::Value = serde_json::from_str(trimmed).expect("stdout should be json");

    assert!(parsed["functions"].is_array());
    assert!(
        !trimmed.contains("compile: ok"),
        "dump mode should emit MIR json instead of compile status line"
    );
}
