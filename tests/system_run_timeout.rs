mod common;

use std::fs;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn system_run_timeout_ms_returns_timeout_result_without_hanging() {
    let fixture_root = common::unique_fixture_root("system-run-timeout");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    result = System.run(\"printf 'before\\n'; sleep 5; printf 'after\\n'\", %{timeout_ms: 150, stream: true})\n    IO.puts(\"exit=#{Map.get(result, :exit_code, -1)}\")\n    IO.puts(\"timed_out=#{Map.get(result, :timed_out, false)}\")\n    IO.puts(\"output=#{Map.get(result, :output, \"\")}\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let started_at = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should finish");
    let elapsed = started_at.elapsed();

    assert!(output.status.success(), "expected success, got {output:?}");
    assert!(
        elapsed < Duration::from_secs(3),
        "timeout run should return quickly, got {elapsed:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("before\n"),
        "expected pre-timeout output, got: {stdout:?}"
    );
    assert!(
        stdout.contains("exit=124\n"),
        "expected timeout exit code, got: {stdout:?}"
    );
    assert!(
        stdout.contains("timed_out=true\n"),
        "expected timeout flag, got: {stdout:?}"
    );
    assert!(
        stdout.contains("output=before\n"),
        "expected partial captured output, got: {stdout:?}"
    );
}
