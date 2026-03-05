use std::fs;
use std::process::Command;

mod common;

const TOML: &str = "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n";

fn write_fixture(root: &std::path::Path, source: &str) {
    let src = root.join("src");
    fs::create_dir_all(&src).expect("fixture src dir");
    fs::write(root.join("tonic.toml"), TOML).expect("tonic.toml");
    fs::write(src.join("main.tn"), source).expect("main.tn");
}

fn run_tonic(root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(root)
        .args(["run", "."])
        .output()
        .expect("tonic run command should execute")
}

// ── Arity errors ───────────────────────────────────────────────────────────
//
// Note: calling System.http_read_request with wrong arity is caught by the
// Tonic type-checker before the host function's arity guard is reached.
// The host-function arity contract is exercised in the unit tests in interop.rs.

#[test]
fn run_http_read_request_rejects_wrong_arity_has_deterministic_error() {
    let root = common::unique_fixture_root("http-read-arity");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_read_request()
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for wrong arity"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("arity") || stderr.contains("expects exactly"),
        "expected deterministic arity-related error, got: {stderr}"
    );
}

// ── Type errors ────────────────────────────────────────────────────────────

#[test]
fn run_http_read_request_rejects_int_connection_id_deterministically() {
    let root = common::unique_fixture_root("http-read-type");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_read_request(42)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for int connection_id"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_read_request expects string argument 1; found int"
        ),
        "expected deterministic type error, got: {stderr}"
    );
}

// ── Unknown handle errors ──────────────────────────────────────────────────

#[test]
fn run_http_read_request_rejects_unknown_connection_id_deterministically() {
    let root = common::unique_fixture_root("http-read-unknown");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_read_request("conn:no-such-connection")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for unknown connection_id"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_read_request unknown connection_id: conn:no-such-connection"
        ),
        "expected deterministic unknown-connection error, got: {stderr}"
    );
}
