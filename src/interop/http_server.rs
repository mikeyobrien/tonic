//! HTTP server primitives for Tonic system interop.
//!
//! Provides four host functions for building HTTP/1.1 servers in Tonic code:
//! - `sys_http_listen(host, port)` → binds a TCP listener, returns opaque listener handle
//! - `sys_http_accept(listener_id, timeout_ms)` → accepts one connection, returns opaque connection handle
//! - `sys_http_read_request(connection_id)` → reads and parses an HTTP/1.1 request from a connection
//! - `sys_http_write_response(connection_id, status, headers, body)` → writes an HTTP/1.1 response and closes the connection
//!
//! ## Limitations (not implemented)
//! - HTTP/2 is not supported; only HTTP/1.1 and HTTP/1.0 requests are parsed.
//! - Chunked transfer encoding is not supported; body size is determined by Content-Length.
//! - Keep-alive is not supported; connections are closed after `sys_http_write_response`.
//! - IPv6 addresses are accepted as host strings but behaviour depends on OS support.
//! - Maximum request body size is 8 MB; larger bodies are rejected with a deterministic error.
//! - Maximum header-read timeout is 30 seconds (hardcoded).

use super::system::{
    expect_exact_args, expect_int_arg, expect_list_arg, expect_string_arg, map_with_atom_keys,
    tuple_string_pair,
};
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

// ── Constants ──────────────────────────────────────────────────────────────

const HTTP_ACCEPT_TIMEOUT_MAX_MS: i64 = 3_600_000; // 1 hour
const HTTP_REQUEST_MAX_BODY_BYTES: usize = 8_388_608; // 8 MB
const HTTP_REQUEST_HEADER_TIMEOUT_MS: u64 = 30_000; // 30 s
const HTTP_RESPONSE_WRITE_TIMEOUT_MS: u64 = 30_000; // 30 s

// ── Global state ───────────────────────────────────────────────────────────

/// Monotonic counter for generating unique handle strings.
static HANDLE_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Process-scoped map of listener handles → TcpListeners.
static LISTENER_MAP: LazyLock<Mutex<HashMap<String, TcpListener>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Process-scoped map of connection handles → TcpStreams.
static CONNECTION_MAP: LazyLock<Mutex<HashMap<String, TcpStream>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn next_handle_id() -> u64 {
    HANDLE_COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn allocate_listener(listener: TcpListener) -> String {
    let id = next_handle_id();
    let handle = format!("listener:{id}");
    LISTENER_MAP
        .lock()
        .unwrap()
        .insert(handle.clone(), listener);
    handle
}

fn clone_listener(id: &str) -> Option<TcpListener> {
    let map = LISTENER_MAP.lock().unwrap();
    map.get(id).and_then(|l| l.try_clone().ok())
}

fn allocate_connection(stream: TcpStream) -> String {
    let id = next_handle_id();
    let handle = format!("conn:{id}");
    CONNECTION_MAP
        .lock()
        .unwrap()
        .insert(handle.clone(), stream);
    handle
}

fn clone_connection(id: &str) -> Option<TcpStream> {
    let map = CONNECTION_MAP.lock().unwrap();
    map.get(id).and_then(|s| s.try_clone().ok())
}

fn take_connection(id: &str) -> Option<TcpStream> {
    CONNECTION_MAP.lock().unwrap().remove(id)
}

// ── URL decoding ───────────────────────────────────────────────────────────

/// Decodes percent-encoded sequences (%HH) in a URL path segment.
/// Returns `None` if the input contains an invalid %HH sequence.
fn url_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = hex_nibble(bytes[i + 1])?;
            let lo = hex_nibble(bytes[i + 2])?;
            out.push((hi << 4) | lo);
            i += 3;
        } else if bytes[i] == b'+' {
            out.push(b' ');
            i += 1;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ── HTTP status reason phrases ─────────────────────────────────────────────

fn status_reason(code: i64) -> &'static str {
    match code {
        100 => "Continue",
        101 => "Switching Protocols",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        206 => "Partial Content",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        422 => "Unprocessable Content",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "Unknown",
    }
}

// ── Primitive: sys_http_listen ─────────────────────────────────────────────

/// `sys_http_listen(host: String, port: Int) → %{status: :ok, listener_id: String}`
///
/// Binds a TCP listener on `host:port` and returns an opaque listener handle.
/// On I/O failure (permission denied, port in use, etc.) returns `Err(HostError)`.
fn host_sys_http_listen(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_http_listen", args, 2)?;
    let host = expect_string_arg("sys_http_listen", args, 0)?;
    let port = expect_int_arg("sys_http_listen", args, 1)?;

    // Port 0 is valid: the OS assigns a free port (standard Unix practice).
    // Negative values and values above 65535 are rejected.
    if !(0..=65535).contains(&port) {
        return Err(HostError::new(format!(
            "sys_http_listen port out of range: {port}"
        )));
    }

    let addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&addr).map_err(|e| match e.kind() {
        std::io::ErrorKind::PermissionDenied => HostError::new(format!(
            "sys_http_listen failed to bind {addr}: permission denied"
        )),
        std::io::ErrorKind::AddrInUse => HostError::new(format!(
            "sys_http_listen failed to bind {addr}: address already in use"
        )),
        _ => HostError::new(format!("sys_http_listen failed to bind {addr}: {e}")),
    })?;

    let listener_id = allocate_listener(listener);

    Ok(map_with_atom_keys(vec![
        ("status", RuntimeValue::Atom("ok".to_string())),
        ("listener_id", RuntimeValue::String(listener_id)),
    ]))
}

// ── Primitive: sys_http_accept ─────────────────────────────────────────────

/// `sys_http_accept(listener_id: String, timeout_ms: Int) → %{status: :ok, connection_id, client_ip, client_port}`
///
/// Accepts one incoming TCP connection on the named listener.
/// `timeout_ms = 0` blocks indefinitely; `timeout_ms > 0` waits up to that duration.
/// On timeout, returns `Err(HostError)` with message "sys_http_accept accept timeout elapsed".
fn host_sys_http_accept(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_http_accept", args, 2)?;
    let listener_id = expect_string_arg("sys_http_accept", args, 0)?;
    let timeout_ms = expect_int_arg("sys_http_accept", args, 1)?;

    if timeout_ms < 0 {
        return Err(HostError::new(format!(
            "sys_http_accept timeout_ms must be >= 0, found {timeout_ms}"
        )));
    }
    if timeout_ms > HTTP_ACCEPT_TIMEOUT_MAX_MS {
        return Err(HostError::new(format!(
            "sys_http_accept timeout_ms out of range: {timeout_ms}"
        )));
    }

    // Clone the listener so we release the global map lock before blocking.
    let listener = clone_listener(&listener_id).ok_or_else(|| {
        HostError::new(format!(
            "sys_http_accept unknown listener_id: {listener_id}"
        ))
    })?;

    // Accept one connection, respecting the requested timeout.
    // TcpListener has no set_read_timeout, so we use a thread + channel for the
    // timed case; for timeout_ms == 0 we block directly (no overhead).
    let (stream, peer_addr) = if timeout_ms == 0 {
        listener
            .accept()
            .map_err(|e| HostError::new(format!("sys_http_accept failed: {e}")))?
    } else {
        use std::sync::mpsc;
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(listener.accept());
        });
        rx.recv_timeout(Duration::from_millis(timeout_ms as u64))
            .map_err(|e| match e {
                mpsc::RecvTimeoutError::Timeout => {
                    HostError::new("sys_http_accept accept timeout elapsed".to_string())
                }
                mpsc::RecvTimeoutError::Disconnected => {
                    HostError::new("sys_http_accept failed: internal error".to_string())
                }
            })?
            .map_err(|e| HostError::new(format!("sys_http_accept failed: {e}")))?
    };

    let (client_ip, client_port) = match peer_addr {
        std::net::SocketAddr::V4(addr) => (addr.ip().to_string(), addr.port() as i64),
        std::net::SocketAddr::V6(addr) => (addr.ip().to_string(), addr.port() as i64),
    };

    let connection_id = allocate_connection(stream);

    Ok(map_with_atom_keys(vec![
        ("status", RuntimeValue::Atom("ok".to_string())),
        ("connection_id", RuntimeValue::String(connection_id)),
        ("client_ip", RuntimeValue::String(client_ip)),
        ("client_port", RuntimeValue::Int(client_port)),
    ]))
}

// ── Primitive: sys_http_read_request ──────────────────────────────────────

/// `sys_http_read_request(connection_id: String) → %{status, method, path, query_string, headers, body}`
///
/// Reads and parses a complete HTTP/1.1 request from the named connection.
/// Blocks until the full request is received or the 30-second header timeout elapses.
/// Body is read up to `Content-Length` bytes; bodies exceeding 8 MB are rejected.
/// Supported methods: GET, POST, PUT, PATCH, DELETE, HEAD.
fn host_sys_http_read_request(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_http_read_request", args, 1)?;
    let connection_id = expect_string_arg("sys_http_read_request", args, 0)?;

    // Clone the stream to read without holding the global map lock.
    let stream = clone_connection(&connection_id).ok_or_else(|| {
        HostError::new(format!(
            "sys_http_read_request unknown connection_id: {connection_id}"
        ))
    })?;

    stream
        .set_read_timeout(Some(Duration::from_millis(HTTP_REQUEST_HEADER_TIMEOUT_MS)))
        .map_err(|e| HostError::new(format!("sys_http_read_request failed to read: {e}")))?;

    let mut reader = BufReader::new(stream);

    // ── Parse request line ────────────────────────────────────────────────
    let mut request_line = String::new();
    reader.read_line(&mut request_line).map_err(|e| {
        if e.kind() == std::io::ErrorKind::TimedOut || e.kind() == std::io::ErrorKind::WouldBlock {
            HostError::new("sys_http_read_request timeout reading headers".to_string())
        } else {
            HostError::new(format!("sys_http_read_request failed to read: {e}"))
        }
    })?;

    let request_line = request_line.trim_end_matches(['\r', '\n']);
    let parts: Vec<&str> = request_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(HostError::new(format!(
            "sys_http_read_request malformed request line: {request_line}"
        )));
    }

    let raw_method = parts[0];
    let method = raw_method.to_ascii_uppercase();
    if !matches!(
        method.as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD"
    ) {
        return Err(HostError::new(format!(
            "sys_http_read_request unsupported method: {raw_method}"
        )));
    }

    let path_and_query = parts[1];
    let (raw_path, query_string) = match path_and_query.split_once('?') {
        Some((p, q)) => (p, q.to_string()),
        None => (path_and_query, String::new()),
    };
    let path = url_decode(raw_path).ok_or_else(|| {
        HostError::new("sys_http_read_request invalid path in request".to_string())
    })?;

    // ── Parse headers ─────────────────────────────────────────────────────
    let mut headers: Vec<(String, String)> = Vec::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).map_err(|e| {
            if e.kind() == std::io::ErrorKind::TimedOut
                || e.kind() == std::io::ErrorKind::WouldBlock
            {
                HostError::new("sys_http_read_request timeout reading headers".to_string())
            } else {
                HostError::new(format!("sys_http_read_request failed to read: {e}"))
            }
        })?;

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if trimmed.is_empty() {
            break; // blank line signals end of headers
        }

        let (name, value) = trimmed.split_once(':').ok_or_else(|| {
            HostError::new(format!("sys_http_read_request malformed header: {trimmed}"))
        })?;
        headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
    }

    // ── Read body ─────────────────────────────────────────────────────────
    let body = if matches!(method.as_str(), "GET" | "HEAD" | "DELETE") {
        String::new()
    } else {
        let content_length = headers
            .iter()
            .find(|(k, _)| k == "content-length")
            .and_then(|(_, v)| v.parse::<usize>().ok())
            .unwrap_or(0);

        if content_length > HTTP_REQUEST_MAX_BODY_BYTES {
            return Err(HostError::new(
                "sys_http_read_request request body exceeded max size".to_string(),
            ));
        }

        let mut body_buf = vec![0u8; content_length];
        if content_length > 0 {
            use std::io::Read;
            reader.read_exact(&mut body_buf).map_err(|e| {
                HostError::new(format!("sys_http_read_request failed to read: {e}"))
            })?;
        }
        String::from_utf8_lossy(&body_buf).into_owned()
    };

    // ── Build result map ──────────────────────────────────────────────────
    let header_list = RuntimeValue::List(
        headers
            .into_iter()
            .map(|(k, v)| tuple_string_pair(k, v))
            .collect(),
    );

    Ok(map_with_atom_keys(vec![
        ("status", RuntimeValue::Atom("ok".to_string())),
        ("method", RuntimeValue::String(method)),
        ("path", RuntimeValue::String(path)),
        ("query_string", RuntimeValue::String(query_string)),
        ("headers", header_list),
        ("body", RuntimeValue::String(body)),
    ]))
}

// ── Primitive: sys_http_write_response ────────────────────────────────────

/// `sys_http_write_response(connection_id: String, status: Int, headers: List, body: String) → Bool`
///
/// Writes a complete HTTP/1.1 response to the named connection, then closes the connection.
/// `headers` must be a list of `{string, string}` tuples.
/// `Content-Length` is auto-inserted unless the caller provides it explicitly.
/// Returns `true` on success.
fn host_sys_http_write_response(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_http_write_response", args, 4)?;
    let connection_id = expect_string_arg("sys_http_write_response", args, 0)?;
    let status = expect_int_arg("sys_http_write_response", args, 1)?;
    let headers_list = expect_list_arg("sys_http_write_response", args, 2)?;
    let body = expect_string_arg("sys_http_write_response", args, 3)?;

    if !(100..=599).contains(&status) {
        return Err(HostError::new(format!(
            "sys_http_write_response status code out of range: {status}"
        )));
    }

    // Validate and parse headers list.
    let mut headers: Vec<(String, String)> = Vec::with_capacity(headers_list.len());
    for (index, entry) in headers_list.iter().enumerate() {
        let RuntimeValue::Tuple(name_val, value_val) = entry else {
            return Err(HostError::new(format!(
                "sys_http_write_response headers argument 3 entry {} must be {{string, string}}; found {}",
                index + 1,
                host_value_kind(entry)
            )));
        };
        let RuntimeValue::String(name) = name_val.as_ref() else {
            return Err(HostError::new(format!(
                "sys_http_write_response headers argument 3 entry {} expects string header name; found {}",
                index + 1,
                host_value_kind(name_val.as_ref())
            )));
        };
        let RuntimeValue::String(value) = value_val.as_ref() else {
            return Err(HostError::new(format!(
                "sys_http_write_response headers argument 3 entry {} expects string header value; found {}",
                index + 1,
                host_value_kind(value_val.as_ref())
            )));
        };
        headers.push((name.clone(), value.clone()));
    }

    // Remove the connection from the map — we own it now and will close it after writing.
    let mut stream = take_connection(&connection_id).ok_or_else(|| {
        HostError::new(format!(
            "sys_http_write_response unknown connection_id: {connection_id}"
        ))
    })?;

    stream
        .set_write_timeout(Some(Duration::from_millis(HTTP_RESPONSE_WRITE_TIMEOUT_MS)))
        .map_err(|e| HostError::new(format!("sys_http_write_response failed to write: {e}")))?;

    let reason = status_reason(status);

    // ── Write status line ─────────────────────────────────────────────────
    write!(stream, "HTTP/1.1 {status} {reason}\r\n")
        .map_err(|e| HostError::new(format!("sys_http_write_response failed to write: {e}")))?;

    // ── Write caller-supplied headers ─────────────────────────────────────
    let has_content_length = headers
        .iter()
        .any(|(k, _)| k.eq_ignore_ascii_case("content-length"));

    for (name, value) in &headers {
        write!(stream, "{name}: {value}\r\n")
            .map_err(|e| HostError::new(format!("sys_http_write_response failed to write: {e}")))?;
    }

    // ── Auto-insert Content-Length if not provided ────────────────────────
    if !has_content_length {
        write!(stream, "Content-Length: {}\r\n", body.len())
            .map_err(|e| HostError::new(format!("sys_http_write_response failed to write: {e}")))?;
    }

    // ── End of headers ────────────────────────────────────────────────────
    stream
        .write_all(b"\r\n")
        .map_err(|e| HostError::new(format!("sys_http_write_response failed to write: {e}")))?;

    // ── Write body ────────────────────────────────────────────────────────
    if !body.is_empty() {
        stream
            .write_all(body.as_bytes())
            .map_err(|e| HostError::new(format!("sys_http_write_response failed to write: {e}")))?;
    }

    stream
        .flush()
        .map_err(|e| HostError::new(format!("sys_http_write_response failed to write: {e}")))?;

    // Stream is dropped here, closing the connection.
    Ok(RuntimeValue::Bool(true))
}

// ── Registration ───────────────────────────────────────────────────────────

pub(super) fn register_http_server_host_functions(registry: &HostRegistry) {
    registry.register("sys_http_listen", host_sys_http_listen);
    registry.register("sys_http_accept", host_sys_http_accept);
    registry.register("sys_http_read_request", host_sys_http_read_request);
    registry.register("sys_http_write_response", host_sys_http_write_response);
}

// ── Unit tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
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
}
