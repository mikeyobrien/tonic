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
// Note: calling System.http_write_response with wrong arity is caught by the
// Tonic type-checker before the host function's arity guard is reached.
// The host-function arity contract is exercised in the unit tests in interop.rs.

#[test]
fn run_http_write_response_rejects_wrong_arity_has_deterministic_error() {
    let root = common::unique_fixture_root("http-write-arity");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:1", 200)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for wrong arity");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("arity") || stderr.contains("expects exactly"),
        "expected deterministic arity-related error, got: {stderr}"
    );
}

// ── Type errors ────────────────────────────────────────────────────────────

#[test]
fn run_http_write_response_rejects_string_status_deterministically() {
    let root = common::unique_fixture_root("http-write-type-status");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:1", "200", [], "body")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for string status"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_write_response expects int argument 2; found string"
        ),
        "expected deterministic type error for status, got: {stderr}"
    );
}

#[test]
fn run_http_write_response_rejects_int_headers_deterministically() {
    let root = common::unique_fixture_root("http-write-type-headers");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:1", 200, 42, "body")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for int headers");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_write_response expects list argument 3; found int"
        ),
        "expected deterministic type error for headers, got: {stderr}"
    );
}

// ── Range errors ───────────────────────────────────────────────────────────

#[test]
fn run_http_write_response_rejects_status_too_high_deterministically() {
    let root = common::unique_fixture_root("http-write-range-high");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:1", 600, [], "body")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for status 600");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("error: host error: sys_http_write_response status code out of range: 600"),
        "expected deterministic range error, got: {stderr}"
    );
}

#[test]
fn run_http_write_response_rejects_status_too_low_deterministically() {
    let root = common::unique_fixture_root("http-write-range-low");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:1", 99, [], "body")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for status 99");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("error: host error: sys_http_write_response status code out of range: 99"),
        "expected deterministic range error, got: {stderr}"
    );
}

// ── Unknown handle errors ──────────────────────────────────────────────────

#[test]
fn run_http_write_response_rejects_unknown_connection_id_deterministically() {
    let root = common::unique_fixture_root("http-write-unknown");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:no-such-connection", 200, [], "hello")
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
            "error: host error: sys_http_write_response unknown connection_id: conn:no-such-connection"
        ),
        "expected deterministic unknown-connection error, got: {stderr}"
    );
}

// ── Header validation ──────────────────────────────────────────────────────

#[test]
fn run_http_write_response_rejects_non_tuple_header_entry_deterministically() {
    let root = common::unique_fixture_root("http-write-header-format");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:1", 200, ["not-a-tuple"], "body")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for non-tuple header entry"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_write_response headers argument 3 entry 1 must be {string, string}; found string"
        ),
        "expected deterministic header-format error, got: {stderr}"
    );
}
