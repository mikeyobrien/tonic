use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::process::{Command, Stdio};

mod common;

#[test]
fn run_system_read_text_reads_file_content() {
    let fixture_root = common::unique_fixture_root("system-read-text");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_text(\"payload.txt\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(fixture_root.join("payload.txt"), "hello from file")
        .expect("fixture setup should write payload");

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

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "\"hello from file\"\n"
    );
}

#[test]
fn run_system_read_stdin_reads_piped_input() {
    let fixture_root = common::unique_fixture_root("system-read-stdin");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_stdin()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run command should spawn");

    let mut stdin = child.stdin.take().expect("stdin pipe should be available");
    stdin
        .write_all(b"piped input")
        .expect("stdin write should succeed");
    drop(stdin);

    let output = child
        .wait_with_output()
        .expect("run command should complete");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "\"piped input\"\n"
    );
}

#[test]
fn run_system_http_request_returns_expected_map_shape() {
    let fixture_root = common::unique_fixture_root("system-http-request-success");
    let src_dir = fixture_root.join("src");

    let listener = TcpListener::bind("127.0.0.1:0").expect("listener should bind");
    let addr = listener
        .local_addr()
        .expect("listener should expose address");
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener
            .accept()
            .expect("server should accept one connection");

        let mut request_buf = [0u8; 1024];
        let _ = stream.read(&mut request_buf);

        let body = "hello";
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-Test: yep\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        stream
            .write_all(response.as_bytes())
            .expect("server should write response");
    });

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        format!(
            "defmodule Demo do\n  def run() do\n    System.http_request(\"GET\", \"http://{addr}/demo\", [], \"\", %{{}})\n  end\nend\n"
        ),
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    server.join().expect("server thread should finish");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains(":status => 200"),
        "expected status key in response map, got: {stdout}"
    );
    assert!(
        stdout.contains(":headers => ["),
        "expected headers list in response map, got: {stdout}"
    );
    assert!(
        stdout.contains("{\"content-type\", \"text/plain\"}"),
        "expected lowercase content-type header tuple, got: {stdout}"
    );
    assert!(
        stdout.contains(":body => \"hello\""),
        "expected body key in response map, got: {stdout}"
    );
    assert!(
        stdout.contains(":final_url => \"http://"),
        "expected final_url key in response map, got: {stdout}"
    );
}

#[test]
fn run_system_read_stdin_returns_empty_string_for_empty_input() {
    let fixture_root = common::unique_fixture_root("system-read-stdin-empty");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_stdin()\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let mut child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run command should spawn");

    // Close stdin immediately without writing — empty input
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .expect("run command should complete");

    assert!(
        output.status.success(),
        "expected run success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "\"\"\n"
    );
}

#[test]
fn run_system_read_text_rejects_non_string_argument_deterministically() {
    let fixture_root = common::unique_fixture_root("system-read-text-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_text(42)\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for wrong argument type"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_read_text expects string argument 1; found int"),
        "expected deterministic type-error message, got: {stderr}"
    );
}

#[test]
fn run_system_http_request_rejects_invalid_method_deterministically() {
    let fixture_root = common::unique_fixture_root("system-http-request-method-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.http_request(\"TRACE\", \"https://example.com\", [], \"\", %{})\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for unsupported method"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_http_request invalid method: TRACE"),
        "expected deterministic invalid-method error, got: {stderr}"
    );
}

#[test]
fn run_system_http_request_rejects_timeout_out_of_range_deterministically() {
    let fixture_root = common::unique_fixture_root("system-http-request-timeout-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.http_request(\"GET\", \"https://example.com\", [], \"\", %{timeout_ms: 10})\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for timeout out of range"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_http_request timeout_ms out of range: 10"),
        "expected deterministic timeout-range error, got: {stderr}"
    );
}

#[test]
fn run_system_http_request_rejects_unsupported_url_scheme_deterministically() {
    let fixture_root = common::unique_fixture_root("system-http-request-scheme-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.http_request(\"GET\", \"ftp://example.com\", [], \"\", %{})\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected run failure for unsupported scheme"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_http_request unsupported url scheme: ftp"),
        "expected deterministic unsupported-scheme error, got: {stderr}"
    );
}
