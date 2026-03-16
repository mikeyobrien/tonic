use super::*;
use crate::interop::HOST_REGISTRY;
use crate::runtime::RuntimeValue;
use std::io::{Read, Write};
use std::net::TcpStream;

fn map_get<'a>(map: &'a RuntimeValue, key: &str) -> Option<&'a RuntimeValue> {
    let RuntimeValue::Map(entries) = map else {
        return None;
    };
    entries.iter().find_map(|(k, v)| {
        if matches!(k, RuntimeValue::Atom(a) if a == key) {
            Some(v)
        } else {
            None
        }
    })
}

fn setup_server_connection() -> (String, TcpStream) {
    let listen_result = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(0),
            ],
        )
        .expect("listen should succeed");
    let listener_id = match map_get(&listen_result, "listener_id") {
        Some(RuntimeValue::String(s)) => s.clone(),
        other => panic!("expected listener_id string, got: {other:?}"),
    };

    let port = {
        let map = LISTENER_MAP.lock().unwrap();
        map.get(&listener_id)
            .and_then(|l| l.local_addr().ok())
            .map(|a| a.port())
            .expect("listener should have a local address")
    };

    let client_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        TcpStream::connect(format!("127.0.0.1:{port}")).expect("client connect should succeed")
    });

    let accept_result = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[RuntimeValue::String(listener_id), RuntimeValue::Int(2000)],
        )
        .expect("accept should succeed");

    let connection_id = match map_get(&accept_result, "connection_id") {
        Some(RuntimeValue::String(s)) => s.clone(),
        other => panic!("expected connection_id string, got: {other:?}"),
    };

    let client_stream = client_thread.join().expect("client thread should finish");
    (connection_id, client_stream)
}

#[test]
fn read_request_url_decodes_path() {
    let (connection_id, mut client) = setup_server_connection();

    client
        .write_all(b"GET /hello%20world HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");
    drop(client);

    let result = HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id)],
        )
        .expect("read_request should succeed");

    assert_eq!(
        map_get(&result, "path"),
        Some(&RuntimeValue::String("/hello world".to_string()))
    );
}

#[test]
fn read_request_parses_post_with_body() {
    let (connection_id, mut client) = setup_server_connection();

    let body = b"name=alice&age=30";
    let request = format!(
        "POST /submit HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    client
        .write_all(request.as_bytes())
        .expect("client write should succeed");
    client
        .write_all(body)
        .expect("client body write should succeed");
    drop(client);

    let result = HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id)],
        )
        .expect("read_request should succeed");

    assert_eq!(
        map_get(&result, "method"),
        Some(&RuntimeValue::String("POST".to_string()))
    );
    assert_eq!(
        map_get(&result, "body"),
        Some(&RuntimeValue::String("name=alice&age=30".to_string()))
    );
}

#[test]
fn read_request_rejects_unsupported_method() {
    let (connection_id, mut client) = setup_server_connection();

    client
        .write_all(b"TRACE /debug HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");
    drop(client);

    let err = HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id)],
        )
        .expect_err("unsupported method should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_read_request unsupported method: TRACE"
    );
}

// ── sys_http_write_response ────────────────────────────────────────────

#[test]
fn write_response_rejects_wrong_arity() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String("conn:1".to_string()),
                RuntimeValue::Int(200),
            ],
        )
        .expect_err("wrong arity should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_write_response expects exactly 4 arguments, found 2"
    );
}

#[test]
fn write_response_rejects_wrong_type_for_status() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String("conn:1".to_string()),
                RuntimeValue::String("200".to_string()),
                RuntimeValue::List(Vec::new()),
                RuntimeValue::String(String::new()),
            ],
        )
        .expect_err("string status should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_write_response expects int argument 2; found string"
    );
}

#[test]
fn write_response_rejects_status_out_of_range() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String("conn:1".to_string()),
                RuntimeValue::Int(600),
                RuntimeValue::List(Vec::new()),
                RuntimeValue::String(String::new()),
            ],
        )
        .expect_err("status 600 should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_write_response status code out of range: 600"
    );
}

#[test]
fn write_response_rejects_invalid_header_entry() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String("conn:1".to_string()),
                RuntimeValue::Int(200),
                RuntimeValue::List(vec![RuntimeValue::String("not-a-tuple".to_string())]),
                RuntimeValue::String(String::new()),
            ],
        )
        .expect_err("string header entry should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_write_response headers argument 3 entry 1 must be {string, string}; found string"
    );
}

#[test]
fn write_response_rejects_unknown_connection_id() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String("conn:does-not-exist".to_string()),
                RuntimeValue::Int(200),
                RuntimeValue::List(Vec::new()),
                RuntimeValue::String("hello".to_string()),
            ],
        )
        .expect_err("unknown connection_id should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_write_response unknown connection_id: conn:does-not-exist"
    );
}

#[test]
fn write_response_sends_200_ok_with_body() {
    let (connection_id, mut client) = setup_server_connection();

    // Client sends request; server must read it before writing the response.
    // Without reading the request, closing the socket with unread data sends RST
    // instead of FIN, which can cause the client's read_to_string to fail.
    client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");

    HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id.clone())],
        )
        .expect("read_request should succeed before writing response");

    let result = HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String(connection_id),
                RuntimeValue::Int(200),
                RuntimeValue::List(Vec::new()),
                RuntimeValue::String("hello world".to_string()),
            ],
        )
        .expect("write_response should succeed");

    assert_eq!(result, RuntimeValue::Bool(true));

    // Read the response on the client side.
    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("client read should succeed");

    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "response should start with status line, got: {response:?}"
    );
    assert!(
        response.contains("Content-Length: 11\r\n"),
        "response should contain auto Content-Length, got: {response:?}"
    );
    assert!(
        response.ends_with("hello world"),
        "response should end with body, got: {response:?}"
    );
}

#[test]
fn write_response_sends_404_not_found() {
    let (connection_id, mut client) = setup_server_connection();

    client
        .write_all(b"GET /missing HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");

    HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id.clone())],
        )
        .expect("read_request should succeed before writing response");

    HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String(connection_id),
                RuntimeValue::Int(404),
                RuntimeValue::List(Vec::new()),
                RuntimeValue::String("not found".to_string()),
            ],
        )
        .expect("write_response should succeed");

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("client read should succeed");

    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "response should have 404 reason phrase, got: {response:?}"
    );
}

#[test]
fn write_response_includes_custom_headers() {
    let (connection_id, mut client) = setup_server_connection();

    client
        .write_all(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");

    HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id.clone())],
        )
        .expect("read_request should succeed before writing response");

    HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String(connection_id),
                RuntimeValue::Int(200),
                RuntimeValue::List(vec![RuntimeValue::Tuple(
                    Box::new(RuntimeValue::String("X-Custom".to_string())),
                    Box::new(RuntimeValue::String("tonic-test".to_string())),
                )]),
                RuntimeValue::String("ok".to_string()),
            ],
        )
        .expect("write_response should succeed");

    let mut response = String::new();
    client
        .read_to_string(&mut response)
        .expect("client read should succeed");

    assert!(
        response.contains("X-Custom: tonic-test\r\n"),
        "response should contain custom header, got: {response:?}"
    );
}

// ── Full request/response cycle ────────────────────────────────────────

#[test]
fn full_request_response_cycle_succeeds() {
    let listen_result = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(0),
            ],
        )
        .expect("listen should succeed");
    let listener_id = match map_get(&listen_result, "listener_id") {
        Some(RuntimeValue::String(s)) => s.clone(),
        other => panic!("expected listener_id, got: {other:?}"),
    };

    let port = {
        let map = LISTENER_MAP.lock().unwrap();
        map.get(&listener_id)
            .and_then(|l| l.local_addr().ok())
            .map(|a| a.port())
            .expect("listener should have port")
    };

    // Client thread: connect, send GET, read response.
    let client_thread = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(10));
        let mut stream =
            TcpStream::connect(format!("127.0.0.1:{port}")).expect("client should connect");
        stream
            .write_all(b"GET /ping HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .expect("client should write request");
        let mut response = String::new();
        stream
            .read_to_string(&mut response)
            .expect("client should read response");
        response
    });

    // Server: accept, read request, write response.
    let accept_result = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[RuntimeValue::String(listener_id), RuntimeValue::Int(2000)],
        )
        .expect("accept should succeed");
    let connection_id = match map_get(&accept_result, "connection_id") {
        Some(RuntimeValue::String(s)) => s.clone(),
        other => panic!("expected connection_id, got: {other:?}"),
    };

    let req = HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id.clone())],
        )
        .expect("read_request should succeed");
    assert_eq!(
        map_get(&req, "path"),
        Some(&RuntimeValue::String("/ping".to_string()))
    );

    HOST_REGISTRY
        .call(
            "sys_http_write_response",
            &[
                RuntimeValue::String(connection_id),
                RuntimeValue::Int(200),
                RuntimeValue::List(Vec::new()),
                RuntimeValue::String("pong".to_string()),
            ],
        )
        .expect("write_response should succeed");

    let response = client_thread.join().expect("client thread should finish");
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.ends_with("pong"));
}
