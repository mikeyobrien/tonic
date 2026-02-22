use std::fs;
use std::path::{Path, PathBuf};

use std::os::unix::fs::PermissionsExt;

#[test]
#[cfg(unix)]
fn run_succeeds_and_warns_when_cache_directory_is_unwritable() {
    let fixture_root = unique_fixture_root("run-cache-permission-denied");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        r#"[project]
name = "demo"
entry = "src/main.tn"
"#,
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do
  def run() do
    1
  end
end
",
    )
    .expect("fixture setup should write entry module source");

    // Create the cache dir but make it read-only
    let cache_dir = fixture_root.join(".tonic").join("cache");
    fs::create_dir_all(&cache_dir).unwrap();
    let mut perms = fs::metadata(&cache_dir).unwrap().permissions();
    perms.set_readonly(true);
    fs::set_permissions(&cache_dir, perms).unwrap();

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
        stderr.contains("Permission denied"),
        "expected permission denied diagnostic, got: {stderr:?}"
    );

    // Restore permissions so cleanup doesn't fail
    let mut perms = fs::metadata(&cache_dir).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&cache_dir, perms).unwrap();
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
