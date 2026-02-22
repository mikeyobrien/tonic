use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn run_succeeds_and_warns_when_cache_directory_is_a_file() {
    let fixture_root = unique_fixture_root("run-cache-path-conflict");
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

    // Inject file where dir should be
    let cache_dir = fixture_root.join(".tonic");
    fs::create_dir_all(&cache_dir).unwrap();
    let cache_dir_cache = cache_dir.join("cache");
    fs::write(&cache_dir_cache, "this is a file, not a dir").unwrap();

    let output = run_with_cache_trace(&fixture_root);
    assert!(
        output.status.success(),
        "expected run to succeed despite cache write failure, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should remain utf8");
    assert!(
        stderr.contains("warning: failed to write cache artifact"),
        "expected warning for failed cache write, got: {stderr:?}"
    );
    assert!(
        stderr.contains("cache-status miss"),
        "expected cache miss to be traced"
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
