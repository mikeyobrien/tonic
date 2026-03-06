/// End-to-end integration tests for the HTTP server primitives.
///
/// Each test spawns a tonic subprocess as the server, then connects to it
/// via a TCP socket from the Rust test thread acting as the HTTP client.
///
/// **Tonic language note:** `def run() do...end` bodies hold a SINGLE expression.
/// Multi-step server logic uses a `with` expression to chain multiple host
/// function calls via `<-` bindings.  The `_wr <-` pattern executes the
/// write_response call as part of the binding sequence, allowing the final
/// `do` body to return a useful value (e.g., `req[:path]`).
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::Command;
use std::time::Duration;
use std::{fs, thread};

mod common;

const TOML: &str = "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n";

fn write_fixture(root: &std::path::Path, source: &str) {
    let src = root.join("src");
    fs::create_dir_all(&src).expect("fixture src dir");
    fs::write(root.join("tonic.toml"), TOML).expect("tonic.toml");
    fs::write(src.join("main.tn"), source).expect("main.tn");
}

/// Allocate a free port by letting the OS choose and then releasing the socket.
/// There is a small TOCTOU window, but this pattern is reliable in practice
/// for loopback-only tests.
fn find_free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("should bind to OS-assigned port")
        .local_addr()
        .expect("should expose local addr")
        .port()
}

/// Connect to `127.0.0.1:port` retrying up to `budget_ms` milliseconds.
fn connect_with_retry(port: u16, budget_ms: u64) -> TcpStream {
    let deadline = std::time::Instant::now() + Duration::from_millis(budget_ms);
    loop {
        match TcpStream::connect(format!("127.0.0.1:{port}")) {
            Ok(stream) => return stream,
            Err(_) if std::time::Instant::now() < deadline => {
                thread::sleep(Duration::from_millis(20));
            }
            Err(e) => panic!("failed to connect to 127.0.0.1:{port} within {budget_ms}ms: {e}"),
        }
    }
}

// ── Full GET request/response cycle ───────────────────────────────────────

#[test]
fn run_http_server_full_get_request_response_cycle() {
    let port = find_free_port();
    let root = common::unique_fixture_root("http-server-cycle-get");

    // Tonic do...end blocks hold a single expression.
    // Use `with` to chain listen → accept → read_request → write_response.
    // The `_wr <-` binding executes write_response; the body returns req[:path].
    write_fixture(
        &root,
        &format!(
            r#"defmodule Demo do
  def run() do
    with lr <- System.http_listen("127.0.0.1", {port}),
         ar <- System.http_accept(lr[:listener_id], 5000),
         req <- System.http_read_request(ar[:connection_id]),
         _wr <- System.http_write_response(ar[:connection_id], 200, [], "pong") do
      req[:path]
    end
  end
end
"#,
            port = port
        ),
    );

    let child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&root)
        .args(["run", "."])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("tonic process should spawn");

    let mut client = connect_with_retry(port, 3000);

    client
        .write_all(b"GET /ping HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client should send request");
    // Signal EOF on the send side so the server's content-length read completes.
    client.shutdown(std::net::Shutdown::Write).ok();

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("client should read response");

    let output = child
        .wait_with_output()
        .expect("tonic process should exit cleanly");

    assert!(
        output.status.success(),
        "tonic server should exit successfully; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "response should start with 200 OK status line, got: {response:?}"
    );
    assert!(
        response.contains("Content-Length: 4\r\n"),
        "response should contain Content-Length: 4 (\"pong\"), got: {response:?}"
    );
    assert!(
        response.ends_with("pong"),
        "response should end with body \"pong\", got: {response:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("\"/ping\""),
        "tonic should print the request path \"/ping\", got: {stdout}"
    );
}

// ── POST request with body ─────────────────────────────────────────────────

#[test]
fn run_http_server_post_request_body_is_parsed() {
    let port = find_free_port();
    let root = common::unique_fixture_root("http-server-cycle-post");

    write_fixture(
        &root,
        &format!(
            r#"defmodule Demo do
  def run() do
    with lr <- System.http_listen("127.0.0.1", {port}),
         ar <- System.http_accept(lr[:listener_id], 5000),
         req <- System.http_read_request(ar[:connection_id]),
         _wr <- System.http_write_response(ar[:connection_id], 201, [], req[:body]) do
      req[:method]
    end
  end
end
"#,
            port = port
        ),
    );

    let child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&root)
        .args(["run", "."])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("tonic process should spawn");

    let mut client = connect_with_retry(port, 3000);

    let body = b"hello=world";
    let request = format!(
        "POST /submit HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    client
        .write_all(request.as_bytes())
        .expect("client should write headers");
    client.write_all(body).expect("client should write body");
    client.shutdown(std::net::Shutdown::Write).ok();

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("client should read response");

    let output = child
        .wait_with_output()
        .expect("tonic process should exit cleanly");

    assert!(
        output.status.success(),
        "tonic server should exit successfully; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        response.starts_with("HTTP/1.1 201 Created\r\n"),
        "response should have 201 Created status line, got: {response:?}"
    );
    assert!(
        response.ends_with("hello=world"),
        "response body should echo request body, got: {response:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        stdout.contains("\"POST\""),
        "tonic should print method \"POST\", got: {stdout}"
    );
}

// ── Custom response headers ────────────────────────────────────────────────

#[test]
fn run_http_server_response_with_custom_header() {
    let port = find_free_port();
    let root = common::unique_fixture_root("http-server-custom-header");

    write_fixture(
        &root,
        &format!(
            r#"defmodule Demo do
  def serve(connection_id) do
    with req <- System.http_read_request(connection_id),
         _wr <- System.http_write_response(
           connection_id, 200,
           [{{"X-Powered-By", "tonic"}}, {{"Content-Type", "text/plain"}}],
           "ok"
         ) do
      req[:path]
    end
  end

  def run() do
    with lr <- System.http_listen("127.0.0.1", {port}),
         ar <- System.http_accept(lr[:listener_id], 5000) do
      serve(ar[:connection_id])
    end
  end
end
"#,
            port = port
        ),
    );

    let child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&root)
        .args(["run", "."])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("tonic process should spawn");

    let mut client = connect_with_retry(port, 3000);
    client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");
    client.shutdown(std::net::Shutdown::Write).ok();

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("client read should succeed");

    let output = child
        .wait_with_output()
        .expect("tonic process should exit cleanly");

    assert!(
        output.status.success(),
        "tonic server should exit successfully; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        response.contains("X-Powered-By: tonic\r\n"),
        "response should contain custom X-Powered-By header, got: {response:?}"
    );
    assert!(
        response.contains("Content-Type: text/plain\r\n"),
        "response should contain Content-Type header, got: {response:?}"
    );
}

// ── Query string preservation ──────────────────────────────────────────────

#[test]
fn run_http_server_query_string_is_preserved() {
    let port = find_free_port();
    let root = common::unique_fixture_root("http-server-query-string");

    write_fixture(
        &root,
        &format!(
            r#"defmodule Demo do
  def run() do
    with lr <- System.http_listen("127.0.0.1", {port}),
         ar <- System.http_accept(lr[:listener_id], 5000),
         req <- System.http_read_request(ar[:connection_id]),
         _wr <- System.http_write_response(ar[:connection_id], 200, [], req[:query_string]) do
      req[:query_string]
    end
  end
end
"#,
            port = port
        ),
    );

    let child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&root)
        .args(["run", "."])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("tonic process should spawn");

    let mut client = connect_with_retry(port, 3000);
    client
        .write_all(b"GET /search?q=tonic&lang=en HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");
    client.shutdown(std::net::Shutdown::Write).ok();

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("client read should succeed");

    let output = child
        .wait_with_output()
        .expect("tonic process should exit cleanly");

    assert!(
        output.status.success(),
        "tonic server should exit successfully; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        response.ends_with("q=tonic&lang=en"),
        "response body should be the raw query string, got: {response:?}"
    );

    let stdout = String::from_utf8(output.stdout).expect("utf8");
    assert!(
        stdout.contains("\"q=tonic&lang=en\""),
        "tonic should print the query string, got: {stdout}"
    );
}
