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
        self.write_request(value);

        let mut line = String::new();
        self.reader
            .read_line(&mut line)
            .expect("response line should read");
        serde_json::from_str(line.trim()).expect("response should parse")
    }

    fn request_frames(&mut self, value: Value) -> Vec<Value> {
        self.write_request(value);

        let mut frames = Vec::new();
        loop {
            let mut line = String::new();
            self.reader
                .read_line(&mut line)
                .expect("response frame should read");
            let frame: Value =
                serde_json::from_str(line.trim()).expect("response frame should parse");
            let done = frame.get("done").and_then(Value::as_bool).unwrap_or(false);
            let is_stream = frame.get("status").and_then(Value::as_str) == Some("stream");
            frames.push(frame);
            if done || !is_stream {
                return frames;
            }
        }
    }

    fn write_request(&mut self, value: Value) {
        serde_json::to_writer(&mut self.writer, &value).expect("request should serialize");
        self.writer
            .write_all(b"\n")
            .expect("request newline should write");
        self.writer.flush().expect("request should flush");
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
fn remote_repl_server_describe_reports_capabilities() {
    let server = spawn_repl_server();
    let mut client = ReplClient::connect(&server.addr);

    let describe = client.request(json!({ "op": "describe" }));
    assert_eq!(describe["status"], "ok");
    assert_eq!(
        describe["describe"]["sessions"]["default_session"],
        "connection"
    );
    assert_eq!(describe["describe"]["sessions"]["logical_sessions"], true);
    assert_eq!(
        describe["describe"]["sessions"]["reconnectable_sessions"],
        true
    );
    assert_eq!(describe["describe"]["sessions"]["clone_op"], "clone");
    assert_eq!(describe["describe"]["sessions"]["close_op"], "close");
    assert_eq!(
        describe["describe"]["ops"]["eval"]["requires"],
        json!(["code"])
    );
    assert_eq!(
        describe["describe"]["ops"]["eval"]["optional"],
        json!(["id", "session", "stdin"])
    );
    assert_eq!(
        describe["describe"]["ops"]["load-file"]["requires"],
        json!(["path"])
    );
    assert_eq!(
        describe["describe"]["ops"]["load-file"]["optional"],
        json!(["id", "session", "stdin"])
    );
    assert_eq!(
        describe["describe"]["ops"]["describe"]["optional"],
        json!(["id"])
    );
    assert_eq!(describe["describe"]["streaming"]["request_ids"], true);
    assert_eq!(
        describe["describe"]["streaming"]["stdout_stderr_frames"],
        true
    );
    assert_eq!(
        describe["describe"]["streaming"]["terminal_done_response"],
        true
    );
    assert!(describe["describe"]["ops"]["describe"].is_object());
}

#[test]
fn remote_repl_server_accepts_request_scoped_stdin_for_eval_and_load_file() {
    let server = spawn_repl_server();
    let mut client = ReplClient::connect(&server.addr);

    let eval = client.request(json!({
        "op": "eval",
        "code": "tuple(host_call(:io_gets, \"prompt> \"), host_call(:sys_read_stdin))",
        "stdin": "typed line\nrest"
    }));
    assert_eq!(eval["status"], "ok");
    assert_eq!(eval["value"], "{\"typed line\", \"rest\"}");
    assert_eq!(eval["value_type"], "{_, _}");
    assert_eq!(eval["stdout"], "prompt> ");

    let file_path = unique_temp_file("repl-server-stdin");
    std::fs::write(
        &file_path,
        "tuple(host_call(:io_gets, \"file> \"), host_call(:sys_read_stdin))\n",
    )
    .expect("stdin fixture should be writable");

    let loaded = client.request(json!({
        "op": "load-file",
        "path": file_path.display().to_string(),
        "stdin": "file line\ntail"
    }));
    assert_eq!(loaded["status"], "ok");
    assert_eq!(loaded["value"], "{\"file line\", \"tail\"}");
    assert_eq!(loaded["value_type"], "{_, _}");
    assert_eq!(loaded["stdout"], "file> ");

    let _ = std::fs::remove_file(file_path);
}

#[test]
fn remote_repl_server_returns_captured_stdout_and_stderr_for_eval_and_load_file() {
    let server = spawn_repl_server();
    let mut client = ReplClient::connect(&server.addr);

    let eval = client.request(json!({
        "op": "eval",
        "code": "case host_call(:io_puts, \"hello\") do\n  _ -> host_call(:sys_log, \"info\", \"remote_eval\", %{source: \"repl\"})\nend"
    }));
    assert_eq!(eval["status"], "ok");
    assert_eq!(eval["value"], "true");
    assert_eq!(eval["value_type"], "bool");
    assert_eq!(eval["stdout"], "hello\n");
    let eval_stderr = eval["stderr"]
        .as_str()
        .expect("eval should capture System.log stderr output");
    assert!(eval_stderr.contains("\"event\":\"remote_eval\""));
    assert!(eval_stderr.contains("\"level\":\"info\""));
    assert!(eval_stderr.contains("\"source\":\"repl\""));

    let file_path = unique_temp_file("repl-server-output");
    std::fs::write(
        &file_path,
        "case host_call(:io_puts, \"loaded\") do\n  _ -> host_call(:sys_log, \"warn\", \"remote_load\", %{path: \"fixture\"})\nend\n",
    )
    .expect("fixture file should be writable");

    let loaded = client.request(json!({
        "op": "load-file",
        "path": file_path.display().to_string()
    }));
    assert_eq!(loaded["status"], "ok");
    assert_eq!(loaded["value"], "true");
    assert_eq!(loaded["value_type"], "bool");
    assert_eq!(loaded["stdout"], "loaded\n");
    let load_stderr = loaded["stderr"]
        .as_str()
        .expect("load-file should capture System.log stderr output");
    assert!(load_stderr.contains("\"event\":\"remote_load\""));
    assert!(load_stderr.contains("\"level\":\"warn\""));
    assert!(load_stderr.contains("\"path\":\"fixture\""));

    let _ = std::fs::remove_file(file_path);
}

#[test]
fn remote_repl_server_streams_stdout_stderr_frames_for_id_addressed_connection_requests() {
    let server = spawn_repl_server();
    let mut client = ReplClient::connect(&server.addr);

    let eval_frames = client.request_frames(json!({
        "op": "eval",
        "id": "req-eval-1",
        "code": "case host_call(:io_puts, \"hello\") do\n  _ -> host_call(:sys_log, \"info\", \"remote_eval\", %{source: \"repl\"})\nend"
    }));
    assert_eq!(eval_frames.len(), 3);
    assert_eq!(eval_frames[0]["status"], "stream");
    assert_eq!(eval_frames[0]["id"], "req-eval-1");
    assert_eq!(eval_frames[0]["stream"], "stdout");
    assert_eq!(eval_frames[0]["text"], "hello\n");
    assert_eq!(eval_frames[1]["stream"], "stderr");
    assert!(eval_frames[1]["text"]
        .as_str()
        .expect("stderr frame should be text")
        .contains("\"event\":\"remote_eval\""));
    let eval_done = &eval_frames[2];
    assert_eq!(eval_done["status"], "ok");
    assert_eq!(eval_done["id"], "req-eval-1");
    assert_eq!(eval_done["done"], true);
    assert_eq!(eval_done["value"], "true");
    assert_eq!(eval_done["value_type"], "bool");
    assert!(eval_done.get("stdout").is_none() || eval_done["stdout"].is_null());
    assert!(eval_done.get("stderr").is_none() || eval_done["stderr"].is_null());

    let file_path = unique_temp_file("repl-server-stream-output");
    std::fs::write(
        &file_path,
        "case host_call(:io_puts, \"loaded\") do\n  _ -> host_call(:sys_log, \"warn\", \"remote_load\", %{path: \"fixture\"})\nend\n",
    )
    .expect("streaming fixture should be writable");

    let load_frames = client.request_frames(json!({
        "op": "load-file",
        "id": "req-load-1",
        "path": file_path.display().to_string()
    }));
    assert_eq!(load_frames.len(), 3);
    assert_eq!(load_frames[0]["stream"], "stdout");
    assert_eq!(load_frames[0]["text"], "loaded\n");
    assert_eq!(load_frames[1]["stream"], "stderr");
    assert!(load_frames[1]["text"]
        .as_str()
        .expect("stderr frame should be text")
        .contains("\"event\":\"remote_load\""));
    let load_done = &load_frames[2];
    assert_eq!(load_done["status"], "ok");
    assert_eq!(load_done["id"], "req-load-1");
    assert_eq!(load_done["done"], true);
    assert_eq!(load_done["value"], "true");
    assert_eq!(load_done["value_type"], "bool");

    let _ = std::fs::remove_file(file_path);
}

#[test]
fn remote_repl_server_streams_stdout_stderr_frames_for_id_addressed_logical_sessions() {
    let server = spawn_repl_server();

    let session_id = {
        let mut client = ReplClient::connect(&server.addr);
        let cloned = client.request(json!({ "op": "clone" }));
        assert_eq!(cloned["status"], "ok");
        cloned["session"]
            .as_str()
            .expect("clone should return a session id")
            .to_string()
    };

    let mut resumed_client = ReplClient::connect(&server.addr);
    let eval_frames = resumed_client.request_frames(json!({
        "op": "eval",
        "id": "req-shared-eval",
        "session": session_id.clone(),
        "code": "case host_call(:io_puts, \"shared\") do\n  _ -> host_call(:sys_log, \"info\", \"shared_eval\", %{scope: \"logical\"})\nend"
    }));
    assert_eq!(eval_frames.len(), 3);
    assert_eq!(eval_frames[0]["session"], session_id);
    assert_eq!(eval_frames[0]["stream"], "stdout");
    assert_eq!(eval_frames[0]["text"], "shared\n");
    assert_eq!(eval_frames[1]["session"], session_id);
    assert_eq!(eval_frames[1]["stream"], "stderr");
    assert!(eval_frames[1]["text"]
        .as_str()
        .expect("stderr frame should be text")
        .contains("\"event\":\"shared_eval\""));
    let eval_done = &eval_frames[2];
    assert_eq!(eval_done["status"], "ok");
    assert_eq!(eval_done["id"], "req-shared-eval");
    assert_eq!(eval_done["done"], true);
    assert_eq!(eval_done["session"], session_id);

    let file_path = unique_temp_file("repl-server-stream-shared-output");
    std::fs::write(
        &file_path,
        "case host_call(:io_puts, \"shared-load\") do\n  _ -> host_call(:sys_log, \"warn\", \"shared_load\", %{scope: \"logical\"})\nend\n",
    )
    .expect("logical streaming fixture should be writable");

    let load_frames = resumed_client.request_frames(json!({
        "op": "load-file",
        "id": "req-shared-load",
        "session": session_id.clone(),
        "path": file_path.display().to_string()
    }));
    assert_eq!(load_frames.len(), 3);
    assert_eq!(load_frames[0]["session"], session_id);
    assert_eq!(load_frames[0]["stream"], "stdout");
    assert_eq!(load_frames[0]["text"], "shared-load\n");
    assert_eq!(load_frames[1]["session"], session_id);
    assert_eq!(load_frames[1]["stream"], "stderr");
    assert!(load_frames[1]["text"]
        .as_str()
        .expect("stderr frame should be text")
        .contains("\"event\":\"shared_load\""));
    let load_done = &load_frames[2];
    assert_eq!(load_done["status"], "ok");
    assert_eq!(load_done["id"], "req-shared-load");
    assert_eq!(load_done["done"], true);
    assert_eq!(load_done["session"], session_id);

    let _ = std::fs::remove_file(file_path);
}

#[test]
fn remote_repl_server_logical_sessions_accept_request_scoped_stdin_after_reconnect() {
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
    assert_eq!(resumed["value"], "42");

    let stdin_eval = resumed_client.request(json!({
        "op": "eval",
        "session": session_id.clone(),
        "code": "tuple(host_call(:io_gets, \"shared> \"), host_call(:sys_read_stdin))",
        "stdin": "shared line\nshared tail"
    }));
    assert_eq!(stdin_eval["status"], "ok");
    assert_eq!(stdin_eval["session"], session_id);
    assert_eq!(stdin_eval["value"], "{\"shared line\", \"shared tail\"}");
    assert_eq!(stdin_eval["value_type"], "{_, _}");
    assert_eq!(stdin_eval["stdout"], "shared> ");
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
