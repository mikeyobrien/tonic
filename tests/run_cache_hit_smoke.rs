use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn run_trace_reports_cache_miss_then_hit_for_repeated_project_runs() {
    let fixture_root = unique_fixture_root("run-cache-hit-smoke");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture setup should write entry module source");

    let first_output = run_with_cache_trace(&fixture_root);

    assert!(
        first_output.status.success(),
        "expected first run to succeed, got status {:?} and stderr: {}",
        first_output.status.code(),
        String::from_utf8_lossy(&first_output.stderr)
    );

    let first_stdout = String::from_utf8(first_output.stdout).expect("stdout should be utf8");
    assert_eq!(first_stdout, "1\n");

    let first_stderr = String::from_utf8(first_output.stderr).expect("stderr should be utf8");
    assert!(
        first_stderr.contains("cache-status miss"),
        "expected first run cache trace to report miss, got: {first_stderr:?}"
    );

    let second_output = run_with_cache_trace(&fixture_root);

    assert!(
        second_output.status.success(),
        "expected second run to succeed, got status {:?} and stderr: {}",
        second_output.status.code(),
        String::from_utf8_lossy(&second_output.stderr)
    );

    let second_stdout = String::from_utf8(second_output.stdout).expect("stdout should be utf8");
    assert_eq!(second_stdout, "1\n");

    let second_stderr = String::from_utf8(second_output.stderr).expect("stderr should be utf8");
    assert!(
        second_stderr.contains("cache-status hit"),
        "expected second run cache trace to report hit, got: {second_stderr:?}"
    );
}

fn run_with_cache_trace(fixture_root: &Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .env("TONIC_DEBUG_CACHE", "1")
        .args(["run", "."])
        .output()
        .expect("run command should execute")
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
