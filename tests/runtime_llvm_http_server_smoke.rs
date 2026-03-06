/// Native backend parity tests for HTTP server primitives.
///
/// These differential tests assert deterministic interpreter/native behavior for
/// key validation and error contracts (`tonic run` vs `tonic compile`).
///
/// The C backend host dispatch for `sys_http_listen`, `sys_http_accept`,
/// `sys_http_read_request`, and `sys_http_write_response` lives in
/// `src/c_backend/stubs.rs`.
use std::fs;

mod common;

const TOML: &str = "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n";

fn write_fixture(root: &std::path::Path, source: &str) {
    let src = root.join("src");
    fs::create_dir_all(&src).expect("fixture src dir");
    fs::write(root.join("tonic.toml"), TOML).expect("tonic.toml");
    fs::write(src.join("main.tn"), source).expect("main.tn");
}

// ── sys_http_listen parity ─────────────────────────────────────────────────

#[test]
#[ignore = "LLVM backend is experimental; parity tests disabled until source-location support is added"]
fn compiled_runtime_http_listen_port_range_error_matches_interpreter() {
    let root = common::unique_fixture_root("llvm-http-listen-range");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen("127.0.0.1", 99999)
  end
end
"#,
    );

    let tonic = std::path::Path::new(env!("CARGO_BIN_EXE_tonic"));
    if let Err(mismatch) = common::differential::run_differential_fixture(tonic, &root, ".") {
        panic!(
            "interpreter vs native mismatch for http_listen port range error:\n\
             interpreter exit={}, stderr={}\n\
             native exit={}, stderr={}",
            mismatch.interpreter.exit_code,
            mismatch.interpreter.stderr,
            mismatch.native.exit_code,
            mismatch.native.stderr,
        );
    }
}

#[test]
#[ignore = "LLVM backend is experimental; parity tests disabled until source-location support is added"]
fn compiled_runtime_http_listen_type_error_matches_interpreter() {
    let root = common::unique_fixture_root("llvm-http-listen-type");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_listen(127, 8080)
  end
end
"#,
    );

    let tonic = std::path::Path::new(env!("CARGO_BIN_EXE_tonic"));
    if let Err(mismatch) = common::differential::run_differential_fixture(tonic, &root, ".") {
        panic!(
            "interpreter vs native mismatch for http_listen type error:\n\
             interpreter exit={}, stderr={}\n\
             native exit={}, stderr={}",
            mismatch.interpreter.exit_code,
            mismatch.interpreter.stderr,
            mismatch.native.exit_code,
            mismatch.native.stderr,
        );
    }
}

// ── sys_http_accept parity ─────────────────────────────────────────────────

#[test]
#[ignore = "LLVM backend is experimental; parity tests disabled until source-location support is added"]
fn compiled_runtime_http_accept_unknown_listener_matches_interpreter() {
    let root = common::unique_fixture_root("llvm-http-accept-unknown");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_accept("listener:no-such", 100)
  end
end
"#,
    );

    let tonic = std::path::Path::new(env!("CARGO_BIN_EXE_tonic"));
    if let Err(mismatch) = common::differential::run_differential_fixture(tonic, &root, ".") {
        panic!(
            "interpreter vs native mismatch for http_accept unknown listener:\n\
             interpreter exit={}, stderr={}\n\
             native exit={}, stderr={}",
            mismatch.interpreter.exit_code,
            mismatch.interpreter.stderr,
            mismatch.native.exit_code,
            mismatch.native.stderr,
        );
    }
}

// ── sys_http_read_request parity ───────────────────────────────────────────

#[test]
#[ignore = "LLVM backend is experimental; parity tests disabled until source-location support is added"]
fn compiled_runtime_http_read_request_unknown_connection_matches_interpreter() {
    let root = common::unique_fixture_root("llvm-http-read-unknown");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_read_request("conn:no-such")
  end
end
"#,
    );

    let tonic = std::path::Path::new(env!("CARGO_BIN_EXE_tonic"));
    if let Err(mismatch) = common::differential::run_differential_fixture(tonic, &root, ".") {
        panic!(
            "interpreter vs native mismatch for http_read_request unknown connection:\n\
             interpreter exit={}, stderr={}\n\
             native exit={}, stderr={}",
            mismatch.interpreter.exit_code,
            mismatch.interpreter.stderr,
            mismatch.native.exit_code,
            mismatch.native.stderr,
        );
    }
}

// ── sys_http_write_response parity ─────────────────────────────────────────

#[test]
#[ignore = "LLVM backend is experimental; parity tests disabled until source-location support is added"]
fn compiled_runtime_http_write_response_status_range_error_matches_interpreter() {
    let root = common::unique_fixture_root("llvm-http-write-range");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:1", 600, [], "body")
  end
end
"#,
    );

    let tonic = std::path::Path::new(env!("CARGO_BIN_EXE_tonic"));
    if let Err(mismatch) = common::differential::run_differential_fixture(tonic, &root, ".") {
        panic!(
            "interpreter vs native mismatch for http_write_response status range error:\n\
             interpreter exit={}, stderr={}\n\
             native exit={}, stderr={}",
            mismatch.interpreter.exit_code,
            mismatch.interpreter.stderr,
            mismatch.native.exit_code,
            mismatch.native.stderr,
        );
    }
}

#[test]
#[ignore = "LLVM backend is experimental; parity tests disabled until source-location support is added"]
fn compiled_runtime_http_write_response_unknown_connection_matches_interpreter() {
    let root = common::unique_fixture_root("llvm-http-write-unknown");
    write_fixture(
        &root,
        r#"defmodule Demo do
  def run() do
    System.http_write_response("conn:no-such", 200, [], "hello")
  end
end
"#,
    );

    let tonic = std::path::Path::new(env!("CARGO_BIN_EXE_tonic"));
    if let Err(mismatch) = common::differential::run_differential_fixture(tonic, &root, ".") {
        panic!(
            "interpreter vs native mismatch for http_write_response unknown connection:\n\
             interpreter exit={}, stderr={}\n\
             native exit={}, stderr={}",
            mismatch.interpreter.exit_code,
            mismatch.interpreter.stderr,
            mismatch.native.exit_code,
            mismatch.native.stderr,
        );
    }
}
