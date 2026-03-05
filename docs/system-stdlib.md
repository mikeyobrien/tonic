# System Standard Library

The `System` module is an optional stdlib automatically injected when the resolver detects a reference to `System.*` in user code.  All functions are thin wrappers over Rust-backed host functions registered in the `HostRegistry`.

---

## File System

### `System.path_exists(path: String) → Bool`

Returns `true` if `path` exists on the filesystem, `false` otherwise.

```elixir
System.path_exists("/tmp/my-file.txt")   # → true | false
```

### `System.ensure_dir(path: String) → Bool`

Creates `path` and all parent directories.  Returns `true` on success; raises on I/O failure.

```elixir
System.ensure_dir("/tmp/out/nested")   # → true
```

### `System.write_text(path: String, content: String) → Bool`

Writes `content` to `path`, creating or overwriting the file.  Returns `true` on success; raises on I/O failure.

```elixir
System.write_text("/tmp/hello.txt", "hello, world")   # → true
```

### `System.append_text(path: String, content: String) → Bool`

Appends `content` bytes to `path` (creating the file if needed). Parent directories are created automatically. Returns `true` on success.

```elixir
System.append_text("/tmp/audit/events.jsonl", "{\"event\":\"created\"}\n")
```

### `System.write_text_atomic(path: String, content: String) → Bool`

Atomically replaces `path` by writing `content` to a temporary sibling file and renaming it into place. Parent directories are created automatically. Returns `true` on success.

```elixir
System.write_text_atomic("/tmp/state/proposal.json", "{\"status\":\"approved\"}")
```

### `System.lock_acquire(path: String) → Bool`

Attempts to acquire an advisory lock-file at `path` using exclusive create semantics.

- Returns `true` when lock acquisition succeeds.
- Returns `false` when lock file already exists.

```elixir
if System.lock_acquire("/tmp/state/proposal.lock") do
  # perform write/update
end
```

### `System.lock_release(path: String) → Bool`

Releases an advisory lock-file created by `lock_acquire`.

- Returns `true` when a lock file was removed.
- Returns `false` when no lock file existed.

```elixir
System.lock_release("/tmp/state/proposal.lock")
```

**Selected persistence error contracts**

| Condition | Error message |
|-----------|---------------|
| `append_text` arg 1 is not string | `sys_append_text expects string argument 1; found <type>` |
| `write_text_atomic` arg 2 is not string | `sys_write_text_atomic expects string argument 2; found <type>` |
| `lock_acquire` arg 1 is not string | `sys_lock_acquire expects string argument 1; found <type>` |
| append I/O failure | `sys_append_text failed for '<path>': <os_error>` |
| atomic replace I/O failure | `sys_write_text_atomic failed for '<path>': <os_error>` |
| lock release I/O failure | `sys_lock_release failed for '<path>': <os_error>` |

### `System.read_text(path: String) → String`

Reads and returns the full contents of `path`.  Raises if the file cannot be read.

**Error contract**

| Condition | Error message |
|-----------|---------------|
| arg is not a string | `sys_read_text expects string argument 1; found <type>` |
| file not found / I/O error | `sys_read_text failed for '<path>': <os_error>` |

```elixir
content = System.read_text("/tmp/hello.txt")   # → "hello, world"
```

---

## Standard I/O

### `System.read_stdin() → String`

Reads all bytes from stdin until EOF and returns them as a string.

```elixir
input = System.read_stdin()
```

### `System.cwd() → String`

Returns the current working directory.

```elixir
dir = System.cwd()   # → "/home/alice/projects"
```

---

## Process

### `System.run(command: String) → %{exit_code: Int, output: String}`

Runs `command` in a shell (`sh -lc`), capturing stdout and stderr.  Always returns a map; non-zero exit codes are **not** errors.

```elixir
result = System.run("ls /tmp")
result[:exit_code]   # → 0
result[:output]      # → "..."
```

### `System.sleep_ms(delay_ms: Int) → Bool`

Blocks the current process for `delay_ms` milliseconds and returns `true`.

**Bounds:** `delay_ms` ∈ [0, 300_000]

```elixir
System.sleep_ms(250)
```

### `System.retry_plan(status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after) → %{retry, delay_ms, source}`

Computes a deterministic, bounded retry decision for rate-limit and transient failure handling.

**Arguments**

| # | Name | Type | Notes |
|---|------|------|-------|
| 1 | `status_code` | Int | HTTP status code [100, 599] |
| 2 | `attempt` | Int | 1-based current attempt number |
| 3 | `max_attempts` | Int | Maximum attempts allowed [1, 20] |
| 4 | `base_delay_ms` | Int | Backoff base delay [1, 300_000] |
| 5 | `max_delay_ms` | Int | Hard delay cap [1, 300_000], must be ≥ `base_delay_ms` |
| 6 | `jitter_ms` | Int | Deterministic jitter ceiling [0, 60_000], must be ≤ `max_delay_ms` |
| 7 | `retry_after` | String \| nil | Raw `Retry-After` header value (`"120"`, RFC 7231 date, or `nil`) |

**Behavior summary**

- If `attempt >= max_attempts`: returns `%{:retry => false, :delay_ms => 0, :source => :exhausted}`
- Non-retryable status (not `429` and not `5xx`): `%{:retry => false, :delay_ms => 0, :source => :non_retryable}`
- `429` + parseable `retry_after`: uses bounded header delay (`:source => :retry_after`)
- Otherwise uses bounded exponential backoff + deterministic jitter (`:source => :backoff`)

**Selected error contracts**

| Condition | Error message |
|-----------|---------------|
| `delay_ms` outside [0, 300000] for `sleep_ms` | `sys_sleep_ms delay_ms out of range: <value>` |
| `status_code` outside [100, 599] | `sys_retry_plan status out of range: <value>` |
| `attempt` < 1 | `sys_retry_plan attempt must be >= 1, found <value>` |
| `max_attempts` outside [1, 20] | `sys_retry_plan max_attempts out of range: <value>` |
| arg 7 not string/nil | `sys_retry_plan expects string-or-nil argument 7; found <type>` |

```elixir
plan = System.retry_plan(429, 1, 4, 250, 5_000, 0, "120")
plan[:retry]     # → true
plan[:delay_ms]  # → 5000 (bounded by max_delay_ms)
plan[:source]    # → :retry_after
```

### `System.log(level, event, fields) → Bool`

Emits one structured audit/event record as newline-delimited JSON.

**Arguments**

| # | Name | Type | Notes |
|---|------|------|-------|
| 1 | `level` | String \| Atom | Must be one of `debug`, `info`, `warn`, `error` (case-insensitive) |
| 2 | `event` | String \| Atom | Event name; must not be empty |
| 3 | `fields` | Map | Structured payload map with atom/string keys |

`fields` values accept nested maps/lists/tuples/keywords/scalars. Function values are rejected.

**Sink behavior**

- If `TONIC_SYSTEM_LOG_PATH` is set: append JSON lines to that file (create parent dirs on demand).
- If unset: write JSON lines to stderr.

**Selected error contracts**

| Condition | Error message |
|-----------|---------------|
| invalid log level | `sys_log level must be one of debug|info|warn|error; found <value>` |
| empty event | `sys_log event must not be empty` |
| arg 3 not map | `sys_log expects map argument 3; found <type>` |
| non atom/string field key | `sys_log fields key at entry <n> must be atom or string; found <type>` |

```elixir
System.log(:info, "triage.proposal_created", %{
  proposal_id: "prop-123",
  actor_id: "u-456",
  pending_approval: true,
})
# → true
```

### `System.argv() → [String]`

Returns the full `argv` list as passed to the tonic process.

```elixir
args = System.argv()   # → ["tonic", "run", "."]
```

### `System.env(name: String) → String | nil`

Returns the value of environment variable `name`, or `nil` if unset.

```elixir
home = System.env("HOME")   # → "/home/alice" | nil
```

### `System.which(name: String) → String | nil`

Returns the absolute path of `name` on `PATH`, or `nil` if not found.

```elixir
git = System.which("git")   # → "/usr/bin/git" | nil
```

---

## HTTP Client

### `System.http_request(method, url, headers, body, opts) → %{status, headers, body, final_url}`

Performs a synchronous HTTP/1.1 or HTTP/2 request via `reqwest`.

**Arguments**

| # | Name | Type | Notes |
|---|------|------|-------|
| 1 | `method` | String | One of `GET POST PUT PATCH DELETE HEAD` (case-insensitive) |
| 2 | `url` | String | Must begin with `http://` or `https://` |
| 3 | `headers` | List | List of `{name, value}` string tuples |
| 4 | `body` | String | Request body (`""` for bodyless methods) |
| 5 | `opts` | Map | See below |

**Options map keys**

| Key (atom) | Type | Default | Range |
|------------|------|---------|-------|
| `:timeout_ms` | Int | 30 000 | [100, 120 000] |
| `:max_response_bytes` | Int | 2 097 152 | [1, 8 388 608] |
| `:follow_redirects` | Bool | true | — |
| `:max_redirects` | Int | 3 | [0, 5] |

**Return map shape**

```elixir
%{
  :status     => 200,
  :headers    => [{"content-type", "application/json"}, ...],
  :body       => "...",
  :final_url  => "https://..."
}
```

---

## HTTP Server

The following four primitives enable tonic programs to run minimal HTTP/1.1 servers.  They are backed by Rust's `std::net::TcpListener`/`TcpStream` and carry process-scoped opaque handle strings.

**Limitations**

- HTTP/2 is not supported; only HTTP/1.1 and HTTP/1.0 are parsed.
- Chunked transfer encoding is not supported; body size must be specified via `Content-Length`.
- Keep-alive is not supported; connections are closed after `http_write_response`.
- Maximum request body size: **8 MB**; requests exceeding this are rejected.
- Header-read timeout: **30 seconds** (not configurable).
- Response-write timeout: **30 seconds** (not configurable).
- There is no explicit `http_close_listener` — listener handles remain open for the process lifetime.

Parity coverage: interpreter/native differential checks for deterministic error contracts live in `tests/runtime_llvm_http_server_smoke.rs`.

### `System.http_listen(host: String, port: Int) → %{status: :ok, listener_id: String}`

Binds a TCP listener on `host:port`.  Returns a result map on success; raises `HostError` on validation or bind failure.

`port = 0` asks the OS to assign a free port (useful for testing).

**Error contracts**

| Condition | Error message |
|-----------|---------------|
| arg count ≠ 2 | `sys_http_listen expects exactly 2 arguments, found N` |
| arg 0 not string | `sys_http_listen expects string argument 1; found <type>` |
| arg 1 not int | `sys_http_listen expects int argument 2; found <type>` |
| port < 0 or port > 65535 | `sys_http_listen port out of range: <port>` |
| permission denied | `sys_http_listen failed to bind <host>:<port>: permission denied` |
| port already in use | `sys_http_listen failed to bind <host>:<port>: address already in use` |
| other I/O error | `sys_http_listen failed to bind <host>:<port>: <os_error>` |

```elixir
result = System.http_listen("0.0.0.0", 8080)
listener_id = result[:listener_id]   # opaque string, e.g. "listener:1"
```

### `System.http_accept(listener_id: String, timeout_ms: Int) → %{status, connection_id, client_ip, client_port}`

Accepts one incoming connection on `listener_id`.

- `timeout_ms = 0` — blocks indefinitely.
- `timeout_ms > 0` — waits up to that many milliseconds, then raises with "accept timeout elapsed".

**Error contracts**

| Condition | Error message |
|-----------|---------------|
| arg count ≠ 2 | `sys_http_accept expects exactly 2 arguments, found N` |
| arg 0 not string | `sys_http_accept expects string argument 1; found <type>` |
| arg 1 not int | `sys_http_accept expects int argument 2; found <type>` |
| timeout_ms < 0 | `sys_http_accept timeout_ms must be >= 0, found <value>` |
| timeout_ms > 3 600 000 | `sys_http_accept timeout_ms out of range: <value>` |
| unknown listener_id | `sys_http_accept unknown listener_id: <id>` |
| timeout elapsed | `sys_http_accept accept timeout elapsed` |
| other I/O error | `sys_http_accept failed: <os_error>` |

```elixir
conn = System.http_accept(listener_id, 30000)
connection_id = conn[:connection_id]   # opaque string, e.g. "conn:2"
client_ip     = conn[:client_ip]       # "192.168.1.10"
client_port   = conn[:client_port]     # 54321
```

### `System.http_read_request(connection_id: String) → %{status, method, path, query_string, headers, body}`

Reads and parses a complete HTTP/1.1 request from `connection_id`.  Blocks until the full request is received (with a 30-second header timeout).

**Supported methods:** `GET`, `POST`, `PUT`, `PATCH`, `DELETE`, `HEAD`

**Error contracts**

| Condition | Error message |
|-----------|---------------|
| arg count ≠ 1 | `sys_http_read_request expects exactly 1 argument, found N` |
| arg 0 not string | `sys_http_read_request expects string argument 1; found <type>` |
| unknown connection_id | `sys_http_read_request unknown connection_id: <id>` |
| header timeout | `sys_http_read_request timeout reading headers` |
| malformed request line | `sys_http_read_request malformed request line: <line>` |
| unsupported method | `sys_http_read_request unsupported method: <method>` |
| invalid path encoding | `sys_http_read_request invalid path in request` |
| malformed header | `sys_http_read_request malformed header: <line>` |
| body exceeds 8 MB | `sys_http_read_request request body exceeded max size` |
| other I/O error | `sys_http_read_request failed to read: <os_error>` |

```elixir
req = System.http_read_request(connection_id)
req[:method]        # "GET"
req[:path]          # "/search"  (percent-decoded)
req[:query_string]  # "q=hello"  (raw)
req[:headers]       # [{"host", "localhost"}, {"content-type", "text/plain"}, ...]
req[:body]          # ""  (empty for GET/HEAD/DELETE)
```

### `System.http_write_response(connection_id, status, headers, body) → Bool`

Writes an HTTP/1.1 response to `connection_id`, then **closes the connection**.  `Content-Length` is automatically added if not present in `headers`.  Returns `true` on success.

**Arguments**

| # | Name | Type | Notes |
|---|------|------|-------|
| 1 | `connection_id` | String | From `http_accept` |
| 2 | `status` | Int | HTTP status code [100, 599] |
| 3 | `headers` | List | List of `{string, string}` tuples (may be empty `[]`) |
| 4 | `body` | String | Response body (`""` for empty bodies) |

**Error contracts**

| Condition | Error message |
|-----------|---------------|
| arg count ≠ 4 | `sys_http_write_response expects exactly 4 arguments, found N` |
| arg 0 not string | `sys_http_write_response expects string argument 1; found <type>` |
| arg 1 not int | `sys_http_write_response expects int argument 2; found <type>` |
| arg 2 not list | `sys_http_write_response expects list argument 3; found <type>` |
| arg 3 not string | `sys_http_write_response expects string argument 4; found <type>` |
| status outside [100, 599] | `sys_http_write_response status code out of range: <code>` |
| header entry not a tuple | `sys_http_write_response headers argument 3 entry N must be {string, string}; found <type>` |
| header name not string | `sys_http_write_response headers argument 3 entry N expects string header name; found <type>` |
| header value not string | `sys_http_write_response headers argument 3 entry N expects string header value; found <type>` |
| unknown connection_id | `sys_http_write_response unknown connection_id: <id>` |
| write I/O error | `sys_http_write_response failed to write: <os_error>` |

```elixir
headers = [{"Content-Type", "application/json"}, {"X-Request-Id", "abc123"}]
System.http_write_response(connection_id, 200, headers, "{\"ok\": true}")
# → true   (connection is now closed)
```

### Complete server example

```elixir
defmodule Server do
  def run() do
    result = System.http_listen("0.0.0.0", 8080)
    listener_id = result[:listener_id]

    serve_loop(listener_id)
  end

  def serve_loop(listener_id) do
    conn = System.http_accept(listener_id, 0)    # 0 = blocking
    connection_id = conn[:connection_id]

    req   = System.http_read_request(connection_id)
    body  = "Hello, #{req[:method]} #{req[:path]}!"
    System.http_write_response(connection_id, 200, [], body)

    serve_loop(listener_id)   # accept next connection
  end
end
```

---

## Crypto

### `System.random_token(bytes: Int) → String`

Returns a URL-safe base64url-encoded (no padding) random token of `bytes` random bytes.

**Bounds:** `bytes` ∈ [16, 256]

```elixir
token = System.random_token(32)   # → 43-character base64url string
```

### `System.hmac_sha256_hex(secret: String, message: String) → String`

Computes HMAC-SHA-256 and returns the result as 64 lowercase hex characters.

**Constraints:** Both `secret` and `message` must be non-empty strings.

```elixir
sig = System.hmac_sha256_hex("my-secret", "payload")   # → 64-char hex string
```

### `System.constant_time_eq(left: String, right: String) → Bool`

Compares `left` and `right` using constant-time byte equality and returns `true` only when both values match.

```elixir
System.constant_time_eq("abc123", "abc123")  # → true
System.constant_time_eq("abc123", "abc124")  # → false
```

### `System.discord_ed25519_verify(public_key_hex, signature_hex, timestamp, body) → Bool`

Validates a Discord Ed25519 signature over `timestamp <> body`.

**Arguments**

| # | Name | Type | Notes |
|---|------|------|-------|
| 1 | `public_key_hex` | String | Discord app public key, exactly 64 hex chars (32 bytes) |
| 2 | `signature_hex` | String | `X-Signature-Ed25519`, exactly 128 hex chars (64 bytes) |
| 3 | `timestamp` | String | `X-Signature-Timestamp` |
| 4 | `body` | String | Raw request body string |

Returns `true` when the signature is valid, `false` when verification fails.

**Selected error contracts**

| Condition | Error message |
|-----------|---------------|
| malformed public key length | `sys_discord_ed25519_verify public_key_hex must be 64 hex chars, found <len>` |
| malformed signature length | `sys_discord_ed25519_verify signature_hex must be 128 hex chars, found <len>` |
| non-hex public key/signature chars | `sys_discord_ed25519_verify <field> contains non-hex character at position <n>` |
| invalid public key bytes | `sys_discord_ed25519_verify invalid public_key_hex bytes: <error>` |

```elixir
ok = System.discord_ed25519_verify(
  "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
  "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b",
  "1700000000",
  "{\"type\":1}"
)
```
