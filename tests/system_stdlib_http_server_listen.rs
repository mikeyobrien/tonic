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

// ── Happy path ─────────────────────────────────────────────────────────────

#[test]
fn run_http_listen_returns_ok_map_with_listener_id() {
    let root = common::unique_fixture_root("http-listen-ok");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen("127.0.0.1", 0)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        output.status.success(),
        "expected success, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        stdout.contains(":status => :ok"),
        "expected :status => :ok in output, got: {stdout}"
    );
    assert!(
        stdout.contains(":listener_id => \"listener:"),
        "expected :listener_id => \"listener:... in output, got: {stdout}"
    );
}

// ── Arity errors ───────────────────────────────────────────────────────────
//
// Note: calling System.http_listen with wrong arity is caught by the Tonic
// type-checker/resolver (which knows the stdlib wrapper takes 2 args) before
// the host function's own arity guard is reached.  The deterministic host-
// function arity contract is exercised in the unit tests in interop.rs.
// The CLI test below verifies that some deterministic error is produced.

#[test]
fn run_http_listen_rejects_wrong_arity_has_deterministic_error() {
    let root = common::unique_fixture_root("http-listen-arity");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen("127.0.0.1")
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
fn run_http_listen_rejects_int_host_deterministically() {
    let root = common::unique_fixture_root("http-listen-type-host");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen(127, 8080)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for int host");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("error: host error: sys_http_listen expects string argument 1; found int"),
        "expected deterministic type error for host, got: {stderr}"
    );
}

#[test]
fn run_http_listen_rejects_string_port_deterministically() {
    let root = common::unique_fixture_root("http-listen-type-port");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen("127.0.0.1", "8080")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for string port");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("error: host error: sys_http_listen expects int argument 2; found string"),
        "expected deterministic type error for port, got: {stderr}"
    );
}

// ── Range errors ───────────────────────────────────────────────────────────

#[test]
fn run_http_listen_rejects_port_too_high_deterministically() {
    let root = common::unique_fixture_root("http-listen-range-high");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen("127.0.0.1", 99999)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for port 99999");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("error: host error: sys_http_listen port out of range: 99999"),
        "expected deterministic range error, got: {stderr}"
    );
}

#[test]
fn run_http_listen_rejects_negative_port_deterministically() {
    let root = common::unique_fixture_root("http-listen-range-neg");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen("127.0.0.1", -1)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(!output.status.success(), "expected failure for port -1");
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("error: host error: sys_http_listen port out of range: -1"),
        "expected deterministic negative-port error, got: {stderr}"
    );
}
