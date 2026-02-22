use std::fs;
use std::path::{Path, PathBuf};
mod common;

#[test]
fn run_rebuilds_cache_after_corrupted_artifact_path() {
    let fixture_root = common::unique_fixture_root("run-cache-corruption-recovery");
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

    let warm_output = run_with_cache_trace(&fixture_root);
    assert!(
        warm_output.status.success(),
        "expected warm run to succeed, got status {:?} and stderr: {}",
        warm_output.status.code(),
        String::from_utf8_lossy(&warm_output.stderr)
    );

    let warm_stdout = String::from_utf8(warm_output.stdout).expect("stdout should be utf8");
    assert_eq!(warm_stdout, "1\n");

    let warm_stderr = String::from_utf8(warm_output.stderr).expect("stderr should be utf8");
    assert!(
        warm_stderr.contains("cache-status miss"),
        "expected warm run to report cache miss, got: {warm_stderr:?}"
    );

    let artifact_path = discover_single_cache_artifact_path(&fixture_root);
    fs::remove_file(&artifact_path)
        .expect("test setup should remove warm cache artifact before corruption injection");
    fs::create_dir_all(&artifact_path)
        .expect("test setup should replace cache artifact file with a directory");

    let corrupted_output = run_with_cache_trace(&fixture_root);
    assert!(
        corrupted_output.status.success(),
        "expected run to succeed after cache corruption, got status {:?} and stderr: {}",
        corrupted_output.status.code(),
        String::from_utf8_lossy(&corrupted_output.stderr)
    );

    let corrupted_stdout =
        String::from_utf8(corrupted_output.stdout).expect("stdout should remain utf8");
    assert_eq!(corrupted_stdout, "1\n");

    let corrupted_stderr =
        String::from_utf8(corrupted_output.stderr).expect("stderr should remain utf8");
    assert!(
        corrupted_stderr.contains("cache-status miss"),
        "expected corrupted run to fall back to compile path, got: {corrupted_stderr:?}"
    );

    let recovered_output = run_with_cache_trace(&fixture_root);
    assert!(
        recovered_output.status.success(),
        "expected follow-up run to succeed, got status {:?} and stderr: {}",
        recovered_output.status.code(),
        String::from_utf8_lossy(&recovered_output.stderr)
    );

    let recovered_stdout =
        String::from_utf8(recovered_output.stdout).expect("stdout should remain utf8");
    assert_eq!(recovered_stdout, "1\n");

    let recovered_stderr =
        String::from_utf8(recovered_output.stderr).expect("stderr should remain utf8");
    assert!(
        recovered_stderr.contains("cache-status hit"),
        "expected cache to recover and report hit after corrupted artifact fallback, got: {recovered_stderr:?}"
    );
}

fn discover_single_cache_artifact_path(fixture_root: &Path) -> PathBuf {
    let cache_dir = fixture_root.join(".tonic/cache");
    let mut paths = fs::read_dir(&cache_dir)
        .expect("warm run should create cache directory")
        .map(|entry| {
            entry
                .expect("cache directory entries should be readable")
                .path()
        })
        .collect::<Vec<_>>();

    paths.sort();

    assert_eq!(
        paths.len(),
        1,
        "expected exactly one cache artifact in fixture cache directory"
    );

    paths.remove(0)
}

fn run_with_cache_trace(fixture_root: &Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .env("TONIC_DEBUG_CACHE", "1")
        .args(["run", "."])
        .output()
        .expect("run command should execute")
}
