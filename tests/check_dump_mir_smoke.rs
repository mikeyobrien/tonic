use std::fs;
mod common;

#[test]
fn check_dump_mir_emits_cfg_with_match_and_merge_blocks() {
    let fixture_root = common::unique_fixture_root("check-dump-mir-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("mir_smoke.tn"),
        "defmodule Demo do\n  def run() do\n    case ok(1)? do\n      :ok -> 2\n      _ -> 3\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write MIR smoke source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/mir_smoke.tn", "--dump-mir"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let mir: serde_json::Value = serde_json::from_str(stdout.trim()).expect("mir should be json");

    let blocks = mir["functions"][0]["blocks"]
        .as_array()
        .expect("function should contain blocks");

    assert!(blocks.len() >= 4, "expected cfg blocks for case lowering");
    assert_eq!(blocks[0]["terminator"]["kind"], "match");

    let merge_block = blocks
        .iter()
        .find(|block| {
            block["args"]
                .as_array()
                .is_some_and(|args| !args.is_empty())
        })
        .expect("cfg should include merge block with block args");

    assert_eq!(merge_block["args"][0]["type"], "dynamic");
}
