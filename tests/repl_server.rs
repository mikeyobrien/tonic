use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

struct ReplServerChild {
    child: Child,
    addr: String,
}

impl Drop for ReplServerChild {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct ReplClient {
    reader: BufReader<TcpStream>,
    writer: TcpStream,
}

impl ReplClient {
    fn connect(addr: &str) -> Self {
        let writer = connect_with_retry(addr);
        let reader = BufReader::new(writer.try_clone().expect("stream clone should succeed"));
        Self { reader, writer }
    }

    fn request(&mut self, value: Value) -> Value {
        serde_json::to_writer(&mut self.writer, &value).expect("request should serialize");
        self.writer
            .write_all(b"\n")
            .expect("request newline should write");
        self.writer.flush().expect("request should flush");

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .expect("response line should read");
        serde_json::from_str(line.trim()).expect("response should parse")
    }
}

fn spawn_repl_server() -> ReplServerChild {
    let mut child = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .arg("repl")
        .arg("--listen")
        .arg("127.0.0.1:0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("repl server should start");

    let mut stdout = BufReader::new(child.stdout.take().expect("stdout should be piped"));
    let mut banner = String::new();
    stdout
        .read_line(&mut banner)
        .expect("server banner should be readable");
    let addr = banner
        .trim()
        .strip_prefix("Tonic REPL server listening on ")
        .expect("server banner should include listen address")
        .to_string();

    ReplServerChild { child, addr }
}

fn connect_with_retry(addr: &str) -> TcpStream {
    let deadline = std::time::Instant::now() + Duration::from_secs(3);
    loop {
        match TcpStream::connect(addr) {
            Ok(stream) => return stream,
            Err(err) if std::time::Instant::now() < deadline => {
                let _ = err;
                thread::sleep(Duration::from_millis(25));
            }
            Err(err) => panic!("failed to connect to {addr}: {err}"),
        }
    }
}

fn unique_temp_file(name: &str) -> std::path::PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("tonic-{name}-{nanos}-{}.tn", std::process::id()))
}

#[test]
fn remote_repl_server_persists_state_per_connection_and_supports_clear() {
    let server = spawn_repl_server();

    let mut client_one = ReplClient::connect(&server.addr);
    let define = client_one.request(json!({
        "op": "eval",
        "code": "defmodule Helpers do\n  def double(x) do x * 2 end\nend"
    }));
    assert_eq!(define["status"], "ok");
    assert_eq!(define["value_type"], "nil");

    let persisted = client_one.request(json!({
        "op": "eval",
        "code": "Helpers.double(5)"
    }));
    assert_eq!(persisted["status"], "ok");
    assert_eq!(persisted["value"], "10");
    assert_eq!(persisted["value_type"], "int");

    let mut client_two = ReplClient::connect(&server.addr);
    let isolated = client_two.request(json!({
        "op": "eval",
        "code": "Helpers.double(5)"
    }));
    assert_eq!(isolated["status"], "error");

    let cleared = client_one.request(json!({ "op": "clear" }));
    assert_eq!(cleared["status"], "ok");
    assert_eq!(cleared["message"], "environment cleared");

    let after_clear = client_one.request(json!({
        "op": "eval",
        "code": "Helpers.double(5)"
    }));
    assert_eq!(after_clear["status"], "error");
}

#[test]
fn remote_repl_server_load_file_makes_definitions_available() {
    let server = spawn_repl_server();
    let module_path = unique_temp_file("repl-server-load-file");
    std::fs::write(
        &module_path,
        "defmodule Loaded do\n  def greet() do \"hi\" end\nend\n",
    )
    .expect("module file should be writable");

    let mut client = ReplClient::connect(&server.addr);
    let loaded = client.request(json!({
        "op": "load-file",
        "path": module_path.display().to_string()
    }));
    assert_eq!(loaded["status"], "ok");
    assert_eq!(loaded["value_type"], "nil");

    let invoked = client.request(json!({
        "op": "eval",
        "code": "Loaded.greet()"
    }));
    assert_eq!(invoked["status"], "ok");
    assert_eq!(invoked["value"], "\"hi\"");
    assert_eq!(invoked["value_type"], "string");

    let _ = std::fs::remove_file(module_path);
}

#[test]
fn remote_repl_server_logical_sessions_survive_reconnects_and_support_clone_close() {
    let server = spawn_repl_server();

    let session_id = {
        let mut client = ReplClient::connect(&server.addr);
        let define = client.request(json!({
            "op": "eval",
            "code": "defmodule Sticky do\n  def value() do 41 end\nend"
        }));
        assert_eq!(define["status"], "ok");

        let cloned = client.request(json!({ "op": "clone" }));
        assert_eq!(cloned["status"], "ok");
        cloned["session"]
            .as_str()
            .expect("clone should return a session id")
            .to_string()
    };

    let mut resumed_client = ReplClient::connect(&server.addr);
    let resumed = resumed_client.request(json!({
        "op": "eval",
        "session": session_id.clone(),
        "code": "Sticky.value() + 1"
    }));
    assert_eq!(resumed["status"], "ok");
    assert_eq!(resumed["session"], session_id);
    assert_eq!(resumed["value"], "42");
    assert_eq!(resumed["value_type"], "int");

    let cloned_again = resumed_client.request(json!({
        "op": "clone",
        "session": session_id.clone()
    }));
    assert_eq!(cloned_again["status"], "ok");
    let cloned_session_id = cloned_again["session"]
        .as_str()
        .expect("clone should return a second session id")
        .to_string();
    assert_ne!(cloned_session_id, session_id);

    let closed = resumed_client.request(json!({
        "op": "close",
        "session": session_id.clone()
    }));
    assert_eq!(closed["status"], "ok");
    assert_eq!(closed["message"], "session closed");
    assert_eq!(closed["session"], session_id);

    let closed_eval = resumed_client.request(json!({
        "op": "eval",
        "session": session_id.clone(),
        "code": "Sticky.value()"
    }));
    assert_eq!(closed_eval["status"], "error");
    assert_eq!(closed_eval["session"], session_id);

    let cloned_eval = resumed_client.request(json!({
        "op": "eval",
        "session": cloned_session_id,
        "code": "Sticky.value()"
    }));
    assert_eq!(cloned_eval["status"], "ok");
    assert_eq!(cloned_eval["value"], "41");
}
