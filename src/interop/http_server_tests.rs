use super::*;
use crate::interop::HOST_REGISTRY;
use crate::runtime::RuntimeValue;
use std::io::Write;
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

// ── sys_http_listen ────────────────────────────────────────────────────

#[test]
fn listen_succeeds_on_loopback() {
    let result = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(0), // OS-assigned port
            ],
        )
        .expect("sys_http_listen should succeed on loopback with OS-assigned port");

    assert_eq!(
        map_get(&result, "status"),
        Some(&RuntimeValue::Atom("ok".to_string()))
    );
    assert!(
        matches!(map_get(&result, "listener_id"), Some(RuntimeValue::String(s)) if s.starts_with("listener:")),
        "expected listener_id starting with 'listener:', got: {:?}",
        map_get(&result, "listener_id")
    );
}

#[test]
fn listen_rejects_port_zero_is_valid_os_assigned() {
    // port 0 asks the OS to assign a free port — this must succeed.
    let result = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(0),
            ],
        )
        .expect("port 0 is valid (OS-assigned)");

    assert_eq!(
        map_get(&result, "status"),
        Some(&RuntimeValue::Atom("ok".to_string()))
    );
}

#[test]
fn listen_rejects_port_too_low() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(-1),
            ],
        )
        .expect_err("negative port should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_listen port out of range: -1"
    );
}

#[test]
fn listen_rejects_port_too_high() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(99999),
            ],
        )
        .expect_err("port 99999 should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_listen port out of range: 99999"
    );
}

#[test]
fn listen_rejects_wrong_arity() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[RuntimeValue::String("127.0.0.1".to_string())],
        )
        .expect_err("wrong arity should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_listen expects exactly 2 arguments, found 1"
    );
}

#[test]
fn listen_rejects_wrong_type_for_host() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[RuntimeValue::Int(127), RuntimeValue::Int(8080)],
        )
        .expect_err("int host should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_listen expects string argument 1; found int"
    );
}

#[test]
fn listen_rejects_wrong_type_for_port() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::String("8080".to_string()),
            ],
        )
        .expect_err("string port should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_listen expects int argument 2; found string"
    );
}

#[test]
fn listen_returns_unique_ids_for_two_listeners() {
    let r1 = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(0),
            ],
        )
        .expect("first listen should succeed");
    let r2 = HOST_REGISTRY
        .call(
            "sys_http_listen",
            &[
                RuntimeValue::String("127.0.0.1".to_string()),
                RuntimeValue::Int(0),
            ],
        )
        .expect("second listen should succeed");

    let id1 = map_get(&r1, "listener_id").cloned();
    let id2 = map_get(&r2, "listener_id").cloned();
    assert_ne!(id1, id2, "two listeners must receive different IDs");
}

// ── sys_http_accept ────────────────────────────────────────────────────

#[test]
fn accept_rejects_wrong_arity() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[RuntimeValue::String("listener:99".to_string())],
        )
        .expect_err("wrong arity should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_accept expects exactly 2 arguments, found 1"
    );
}

#[test]
fn accept_rejects_wrong_type_for_listener_id() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[RuntimeValue::Int(123), RuntimeValue::Int(1000)],
        )
        .expect_err("int listener_id should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_accept expects string argument 1; found int"
    );
}

#[test]
fn accept_rejects_negative_timeout() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[
                RuntimeValue::String("listener:1".to_string()),
                RuntimeValue::Int(-1),
            ],
        )
        .expect_err("negative timeout should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_accept timeout_ms must be >= 0, found -1"
    );
}

#[test]
fn accept_rejects_timeout_out_of_range() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[
                RuntimeValue::String("listener:1".to_string()),
                RuntimeValue::Int(HTTP_ACCEPT_TIMEOUT_MAX_MS + 1),
            ],
        )
        .expect_err("timeout above max should be rejected");

    assert_eq!(
        err.to_string(),
        format!(
            "host error: sys_http_accept timeout_ms out of range: {}",
            HTTP_ACCEPT_TIMEOUT_MAX_MS + 1
        )
    );
}

#[test]
fn accept_rejects_unknown_listener_id() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[
                RuntimeValue::String("listener:does-not-exist".to_string()),
                RuntimeValue::Int(100),
            ],
        )
        .expect_err("unknown listener_id should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_accept unknown listener_id: listener:does-not-exist"
    );
}

#[test]
fn accept_times_out_when_no_client_connects() {
    // Bind a listener, then immediately try to accept with a short timeout.
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

    let err = HOST_REGISTRY
        .call(
            "sys_http_accept",
            &[
                RuntimeValue::String(listener_id),
                RuntimeValue::Int(50), // 50ms timeout — no client will connect
            ],
        )
        .expect_err("accept should time out");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_accept accept timeout elapsed"
    );
}

#[test]
fn accept_returns_connection_metadata() {
    // Bind a listener.
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

    // Find the actual bound port by looking up the listener in the map.
    let port = {
        let map = LISTENER_MAP.lock().unwrap();
        map.get(&listener_id)
            .and_then(|l| l.local_addr().ok())
            .map(|a| a.port())
            .expect("listener should have a local address")
    };

    // Connect from a separate thread before accepting.
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

    client_thread.join().expect("client thread should finish");

    assert_eq!(
        map_get(&accept_result, "status"),
        Some(&RuntimeValue::Atom("ok".to_string()))
    );
    assert!(
        matches!(map_get(&accept_result, "connection_id"), Some(RuntimeValue::String(s)) if s.starts_with("conn:")),
        "expected conn: prefix on connection_id"
    );
    assert_eq!(
        map_get(&accept_result, "client_ip"),
        Some(&RuntimeValue::String("127.0.0.1".to_string()))
    );
    assert!(
        matches!(map_get(&accept_result, "client_port"), Some(RuntimeValue::Int(p)) if *p > 0),
        "expected positive client_port"
    );
}

// ── sys_http_read_request ──────────────────────────────────────────────

#[test]
fn read_request_rejects_wrong_arity() {
    let err = HOST_REGISTRY
        .call("sys_http_read_request", &[])
        .expect_err("wrong arity should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_read_request expects exactly 1 argument, found 0"
    );
}

#[test]
fn read_request_rejects_wrong_type() {
    let err = HOST_REGISTRY
        .call("sys_http_read_request", &[RuntimeValue::Int(42)])
        .expect_err("int connection_id should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_read_request expects string argument 1; found int"
    );
}

#[test]
fn read_request_rejects_unknown_connection_id() {
    let err = HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String("conn:does-not-exist".to_string())],
        )
        .expect_err("unknown connection_id should be rejected");

    assert_eq!(
        err.to_string(),
        "host error: sys_http_read_request unknown connection_id: conn:does-not-exist"
    );
}

/// Helper: set up a listener→accept pair and return (connection_id, client_stream).
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
fn read_request_parses_get_request() {
    let (connection_id, mut client) = setup_server_connection();

    client
        .write_all(b"GET /hello HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .expect("client write should succeed");
    drop(client);

    let result = HOST_REGISTRY
        .call(
            "sys_http_read_request",
            &[RuntimeValue::String(connection_id)],
        )
        .expect("read_request should succeed");

    assert_eq!(
        map_get(&result, "method"),
        Some(&RuntimeValue::String("GET".to_string()))
    );
    assert_eq!(
        map_get(&result, "path"),
        Some(&RuntimeValue::String("/hello".to_string()))
    );
    assert_eq!(
        map_get(&result, "query_string"),
        Some(&RuntimeValue::String(String::new()))
    );
    assert_eq!(
        map_get(&result, "body"),
        Some(&RuntimeValue::String(String::new()))
    );
}

#[test]
fn read_request_parses_query_string() {
    let (connection_id, mut client) = setup_server_connection();

    client
        .write_all(b"GET /search?q=hello&lang=en HTTP/1.1\r\nHost: localhost\r\n\r\n")
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
        Some(&RuntimeValue::String("/search".to_string()))
    );
    assert_eq!(
        map_get(&result, "query_string"),
        Some(&RuntimeValue::String("q=hello&lang=en".to_string()))
    );
}
