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
// Note: calling System.http_accept with wrong arity is caught by the Tonic
// type-checker/resolver (which knows the stdlib wrapper takes 2 args) before
// the host function's own arity guard is reached.  The deterministic host-
// function arity contract is exercised in the unit tests in interop.rs.
// The CLI test below verifies that the user-visible error for wrong arity is
// deterministic (even if it comes from the type checker, not the host layer).

#[test]
fn run_http_accept_rejects_wrong_arity_has_deterministic_error() {
    let root = common::unique_fixture_root("http-accept-arity");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_accept("listener:1")
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for wrong arity"
    );
    // The error comes from the Tonic type-checker or call-site resolver.
    // Both the "arity mismatch" and the host-function error are acceptable
    // deterministic contracts at the CLI level.
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("arity") || stderr.contains("expects exactly"),
        "expected deterministic arity-related error, got: {stderr}"
    );
}

// ── Type errors ────────────────────────────────────────────────────────────

#[test]
fn run_http_accept_rejects_int_listener_id_deterministically() {
    let root = common::unique_fixture_root("http-accept-type-id");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_accept(42, 1000)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for int listener_id"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_accept expects string argument 1; found int"
        ),
        "expected deterministic type error for listener_id, got: {stderr}"
    );
}

// ── Range errors ───────────────────────────────────────────────────────────

#[test]
fn run_http_accept_rejects_negative_timeout_deterministically() {
    let root = common::unique_fixture_root("http-accept-neg-timeout");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_accept("listener:1", -1)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for negative timeout"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_accept timeout_ms must be >= 0, found -1"
        ),
        "expected deterministic negative-timeout error, got: {stderr}"
    );
}

#[test]
fn run_http_accept_rejects_timeout_too_large_deterministically() {
    let root = common::unique_fixture_root("http-accept-timeout-large");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_accept("listener:1", 9999999999)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for timeout above max"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_accept timeout_ms out of range: 9999999999"
        ),
        "expected deterministic timeout-range error, got: {stderr}"
    );
}

// ── Unknown handle errors ──────────────────────────────────────────────────

#[test]
fn run_http_accept_rejects_unknown_listener_id_deterministically() {
    let root = common::unique_fixture_root("http-accept-unknown-id");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_accept("listener:no-such-listener", 100)
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure for unknown listener_id"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains(
            "error: host error: sys_http_accept unknown listener_id: listener:no-such-listener"
        ),
        "expected deterministic unknown-listener error, got: {stderr}"
    );
}

// ── Timeout behaviour (functional) ────────────────────────────────────────

#[test]
fn run_http_accept_returns_timeout_error_when_no_client_connects() {
    let root = common::unique_fixture_root("http-accept-timeout-elapsed");
    // Tonic do...end bodies hold a single expression; use `with` to chain
    // listen → accept so the listener_id can be threaded through.
    // No client will connect within 50ms, so we expect "accept timeout elapsed".
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    with lr <- System.http_listen("127.0.0.1", 0) do
      System.http_accept(lr[:listener_id], 50)
    end
  end
end
"#,
    );

    let output = run_tonic(&root);
    assert!(
        !output.status.success(),
        "expected failure when accept times out"
    );
    let stderr = String::from_utf8(output.stderr).expect("utf8");
    assert!(
        stderr.contains("error: host error: sys_http_accept accept timeout elapsed"),
        "expected deterministic timeout-elapsed error, got: {stderr}"
    );
}
