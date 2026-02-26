use std::fs;
use std::process::Command;

mod common;

#[test]
fn run_system_random_token_returns_url_safe_string() {
    let fixture_root = common::unique_fixture_root("system-random-token-success");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.random_token(32)\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    // Output is a quoted string like "abc123_-XYZ"\n
    let trimmed = stdout.trim();
    assert!(
        trimmed.starts_with('"') && trimmed.ends_with('"'),
        "expected quoted string output, got: {trimmed}"
    );

    let token = &trimmed[1..trimmed.len() - 1];
    assert_eq!(
        token.len(),
        43,
        "32 bytes should produce 43 base64url chars, got {} in: {token}",
        token.len()
    );
    assert!(
        token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "token should be base64url safe, got: {token}"
    );
}

#[test]
fn run_system_hmac_sha256_hex_matches_known_vector() {
    let fixture_root = common::unique_fixture_root("system-hmac-sha256-success");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    // RFC 4231 Test Case 2
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.hmac_sha256_hex(\"Jefe\", \"what do ya want for nothing?\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout.trim(),
        "\"5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843\"",
        "HMAC-SHA256 output should match RFC 4231 test vector"
    );
}

#[test]
fn run_system_random_token_rejects_bytes_out_of_range() {
    let fixture_root = common::unique_fixture_root("system-random-token-range-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.random_token(4)\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for bytes out of range"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_random_token bytes out of range: 4"),
        "expected deterministic range error, got: {stderr}"
    );
}

#[test]
fn run_system_hmac_sha256_hex_rejects_empty_secret() {
    let fixture_root = common::unique_fixture_root("system-hmac-sha256-empty-secret-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.hmac_sha256_hex(\"\", \"message\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for empty secret"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_hmac_sha256_hex secret must not be empty"),
        "expected deterministic empty-secret error, got: {stderr}"
    );
}
