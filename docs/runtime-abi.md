# Native Runtime ABI (v1)

This document defines the stable ABI used by native codegen/runtime boundaries.

## Version

- `TONIC_RUNTIME_ABI_VERSION = 1`
- ABI version is carried by `TCallContext.abi_version`.
- Mismatched versions return `TCallStatus::InvalidAbi`.

## TValue layout

`TValue` is `#[repr(C)]` and has fixed v1 layout:

- size: 16 bytes
- alignment: 8 bytes
- fields:
  - `tag: u8` (`TValueTag`)
  - `ownership: u8` (`TOwnership`)
  - `reserved: u16` (must be `0`)
  - `payload: u64`

### Tag and payload policy

Immediate payloads:
- `Int` => two's-complement `i64` in `payload`
- `Bool` => `0` or `1`
- `Nil` => `0`

Heap payloads (refcounted handle IDs):
- `Float`, `String`, `Atom`, `List`, `Map`, `Keyword`, `Tuple2`, `ResultOk`, `ResultErr`, `Closure`, `Range`

Closure handle representation (`Closure` tag):
- payload references a refcounted runtime closure cell
- closure cell contains parameter names, lowered body ops, and captured lexical environment snapshot
- `retain_tvalue` / `release_tvalue` govern closure lifetime across call boundaries

## Memory policy

v1 uses deterministic handle-based reference counting:

- heap values are stored in an internal handle table
- `retain_tvalue` increments refcount
- `release_tvalue` decrements refcount and frees at zero
- releasing an unknown/already-freed handle returns deterministic `AbiErrorCode::InvalidHandle`
- ownership/tag misuse returns deterministic `AbiErrorCode::OwnershipViolation` or `TagHandleMismatch`

### Memory observability (Task 01)

`native_abi::memory_stats_snapshot()` exposes deterministic heap counters:

- `allocations_total`
- `reclaims_total`
- `active_handles`
- `active_handles_high_water`

These counters are monotonic where applicable and are intended for roadmap
baseline capture before collector behavior changes.

## Boundary call ABI

`TCallContext`:
- `abi_version`
- `argc`
- `argv: *const TValue`

`TCallResult`:
- `status: TCallStatus` (`Ok | Err | Panic | InvalidAbi`)
- `value: TValue` (valid for `Ok`)
- `error: TValue` (string payload for `Err/Panic/InvalidAbi`)

`invoke_runtime_boundary(...)` behavior:
- validates ABI version + frame shape
- decodes args via `tvalue_to_runtime`
- catches panics (`catch_unwind`) and returns `Panic` status instead of unwinding across boundary
- maps helper/runtime ABI failures to deterministic `Err` result

## Host interop ABI policy (Task 10)

Host interop introduces a separate compatibility marker for host-function contracts:

- `TONIC_HOST_INTEROP_ABI_VERSION = 1`

Boundary entrypoints for host interop continue to use the same core call ABI:

- `tonic_rt_host_call: extern "C" fn(TCallContext) -> TCallResult`
- `tonic_rt_protocol_dispatch: extern "C" fn(TCallContext) -> TCallResult`

Errors from unknown host functions, key-type mismatch, and arity mismatch are returned as
`TCallStatus::Err` with deterministic string payloads in `TCallResult.error`.

## Host interop function registry

The `HostRegistry` (see `src/interop.rs`) holds all registered host functions.  The following table lists currently registered keys and their arity:

### File system

| Key | Arity | Returns |
|-----|-------|---------|
| `sys_path_exists` | 1 | `Bool` |
| `sys_list_files_recursive` | 1 | `List[String]` — sorted relative paths; symlinks skipped (lstat); errors on missing/empty path |
| `sys_ensure_dir` | 1 | `Bool` |
| `sys_remove_tree` | 1 | `Bool` — `true` removed, `false` not found; symlinks removed as files (lstat); errors on empty path |
| `sys_write_text` | 2 | `Bool` |
| `sys_append_text` | 2 | `Bool` |
| `sys_write_text_atomic` | 2 | `Bool` |
| `sys_lock_acquire` | 1 | `Bool` (`true` when lock file is acquired, `false` when already held) |
| `sys_lock_release` | 1 | `Bool` (`true` when released, `false` when missing) |
| `sys_read_text` | 1 | `String` |
| `sys_read_stdin` | 0 | `String` |

`sys_append_text` appends bytes to the target file and creates parent directories on demand.
`sys_write_text_atomic` writes via a temporary sibling file and `rename` replacement.
`sys_lock_acquire`/`sys_lock_release` provide advisory lock-file semantics for persistence workflows.

### Process

| Key | Arity | Returns |
|-----|-------|---------|
| `sys_run` | 1 | `Map {exit_code, output}` |
| `sys_sleep_ms` | 1 | `Bool` |
| `sys_retry_plan` | 7 | `Map {retry, delay_ms, source}` |
| `sys_log` | 3 | `Bool` |
| `sys_argv` | 0 | `List[String]` |
| `sys_env` | 1 | `String \| Nil` |
| `sys_which` | 1 | `String \| Nil` |
| `sys_cwd` | 0 | `String` |

`sys_log` writes newline-delimited JSON payloads to an append sink. If `TONIC_SYSTEM_LOG_PATH`
is set, the runtime appends to that file (creating parent directories when needed); otherwise
it emits the JSON line to stderr.

### Crypto

| Key | Arity | Returns |
|-----|-------|---------|
| `sys_random_token` | 1 | `String` |
| `sys_hmac_sha256_hex` | 2 | `String` |
| `sys_constant_time_eq` | 2 | `Bool` |
| `sys_discord_ed25519_verify` | 4 | `Bool` |

### HTTP client

| Key | Arity | Returns |
|-----|-------|---------|
| `sys_http_request` | 5 | `Map {status, headers, body, final_url}` |

### HTTP server

HTTP server primitives use **process-scoped opaque handle strings** (`"listener:N"`, `"conn:N"`) as the primary state management mechanism.  Handles are allocated from a global `AtomicU64` counter and stored in `LazyLock<Mutex<HashMap>>` maps.  Handle state is not transferred across process boundaries.

| Key | Arity | Returns |
|-----|-------|---------|
| `sys_http_listen` | 2 | `Map {status: :ok, listener_id}` |
| `sys_http_accept` | 2 | `Map {status: :ok, connection_id, client_ip, client_port}` |
| `sys_http_read_request` | 1 | `Map {status: :ok, method, path, query_string, headers, body}` |
| `sys_http_write_response` | 4 | `Bool` |

**Handle lifecycle**

1. `sys_http_listen` → allocates a `TcpListener`, returns `listener:N`
2. `sys_http_accept` → clones the listener fd, blocks/polls for a connection, allocates a `TcpStream`, returns `conn:M`
3. `sys_http_read_request` → clones the stream fd, reads the HTTP/1.1 request
4. `sys_http_write_response` → **removes** the stream from the map, writes the response, drops the stream (closes connection)

After `sys_http_write_response` the `connection_id` is no longer valid.  The `listener_id` remains valid until the process exits (no explicit close API in v1).

See `docs/system-stdlib.md` for full API reference, error contracts, and examples.

## Conversion helpers

- `runtime_to_tvalue(RuntimeValue) -> Result<TValue, AbiError>`
- `tvalue_to_runtime(TValue) -> Result<RuntimeValue, AbiError>`
- `validate_tvalue(TValue) -> Result<(), AbiError>`

These helpers are used for differential testing between interpreter values and native ABI values.
