use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_pipe_chain_through_enum_style_module_functions() {
    let fixture_root = unique_fixture_root("run-pipe-enum-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_pipe_enum.tn"),
        "defmodule Enum do\n  def stage_one(_value) do\n    1\n  end\n\n  def stage_two(_value) do\n    2\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    tuple(1, 2) |> Enum.stage_one() |> Enum.stage_two()\n  end\nend\n",
    )
    .expect("fixture setup should write pipe + enum source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_pipe_enum.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "2\n");
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}
