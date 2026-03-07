use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use hmac::{Hmac, Mac};
use rand::RngCore;
use reqwest::Method;
use serde_json::{Map as JsonMap, Number as JsonNumber, Value as JsonValue};
use sha2::Sha256;
use std::io::{ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

const RANDOM_TOKEN_MIN_BYTES: i64 = 16;
const RANDOM_TOKEN_MAX_BYTES: i64 = 256;

const HTTP_TIMEOUT_DEFAULT_MS: i64 = 30_000;
const HTTP_TIMEOUT_MIN_MS: i64 = 100;
const HTTP_TIMEOUT_MAX_MS: i64 = 120_000;
const HTTP_MAX_RESPONSE_DEFAULT_BYTES: i64 = 2_097_152;
const HTTP_MAX_RESPONSE_MAX_BYTES: i64 = 8_388_608;
const HTTP_FOLLOW_REDIRECTS_DEFAULT: bool = true;
const HTTP_MAX_REDIRECTS_DEFAULT: i64 = 3;
const HTTP_MAX_REDIRECTS_MAX: i64 = 5;

const SLEEP_MAX_MS: i64 = 300_000;
const RETRY_STATUS_MIN: i64 = 100;
const RETRY_STATUS_MAX: i64 = 599;
const RETRY_MAX_ATTEMPTS_CAP: i64 = 20;
const RETRY_DELAY_MAX_MS: i64 = 300_000;
const RETRY_JITTER_MAX_MS: i64 = 60_000;

const ED25519_PUBLIC_KEY_BYTES: usize = 32;
const ED25519_SIGNATURE_BYTES: usize = 64;
const STRUCTURED_LOG_PATH_ENV: &str = "TONIC_SYSTEM_LOG_PATH";
const STRUCTURED_LOG_LEVELS: [&str; 4] = ["debug", "info", "warn", "error"];

#[derive(Debug, Clone, PartialEq, Eq)]
struct HttpRequestOptions {
    timeout_ms: i64,
    max_response_bytes: i64,
    follow_redirects: bool,
    max_redirects: i64,
}

pub(super) fn expect_exact_args(
    function: &str,
    args: &[RuntimeValue],
    expected: usize,
) -> Result<(), HostError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "{} expects exactly {} argument{}, found {}",
            function,
            expected,
            if expected == 1 { "" } else { "s" },
            args.len()
        )))
    }
}

pub(super) fn expect_string_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<String, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::String(text) => Ok(text.clone()),
        other => Err(HostError::new(format!(
            "{} expects string argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

pub(super) fn expect_int_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<i64, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::Int(n) => Ok(*n),
        other => Err(HostError::new(format!(
            "{} expects int argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

pub(super) fn expect_list_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<Vec<RuntimeValue>, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::List(items) => Ok(items.clone()),
        other => Err(HostError::new(format!(
            "{} expects list argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_map_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<Vec<(RuntimeValue, RuntimeValue)>, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::Map(entries) => Ok(entries.clone()),
        other => Err(HostError::new(format!(
            "{} expects map argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_string_or_atom_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<String, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::String(text) | RuntimeValue::Atom(text) => Ok(text.clone()),
        other => Err(HostError::new(format!(
            "{} expects string-or-atom argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn log_level_label(level: &str) -> Option<&'static str> {
    let normalized = level.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "debug" => Some(STRUCTURED_LOG_LEVELS[0]),
        "info" => Some(STRUCTURED_LOG_LEVELS[1]),
        "warn" => Some(STRUCTURED_LOG_LEVELS[2]),
        "error" => Some(STRUCTURED_LOG_LEVELS[3]),
        _ => None,
    }
}

fn log_field_path(parent_path: &str, segment: &str) -> String {
    if parent_path.is_empty() {
        segment.to_string()
    } else {
        format!("{parent_path}.{segment}")
    }
}

fn runtime_entries_to_json_object(
    function: &str,
    path: &str,
    entries: &[(RuntimeValue, RuntimeValue)],
) -> Result<JsonMap<String, JsonValue>, HostError> {
    let mut object = JsonMap::new();

    for (index, (key, value)) in entries.iter().enumerate() {
        let key_name = match key {
            RuntimeValue::Atom(name) | RuntimeValue::String(name) => name.clone(),
            other => {
                return Err(HostError::new(format!(
                    "{function} {path} key at entry {} must be atom or string; found {}",
                    index + 1,
                    host_value_kind(other)
                )));
            }
        };

        if key_name.trim().is_empty() {
            return Err(HostError::new(format!(
                "{function} {path} key at entry {} must not be empty",
                index + 1
            )));
        }

        let child_path = log_field_path(path, &key_name);
        let json_value = runtime_value_to_json(function, &child_path, value)?;
        object.insert(key_name, json_value);
    }

    Ok(object)
}

fn runtime_value_to_json(
    function: &str,
    path: &str,
    value: &RuntimeValue,
) -> Result<JsonValue, HostError> {
    match value {
        RuntimeValue::Int(number) => Ok(JsonValue::Number((*number).into())),
        RuntimeValue::Float(number_text) => {
            let number = number_text.parse::<f64>().map_err(|_| {
                HostError::new(format!(
                    "{function} {path} float must parse as finite number; found {number_text}"
                ))
            })?;
            let Some(json_number) = JsonNumber::from_f64(number) else {
                return Err(HostError::new(format!(
                    "{function} {path} float must parse as finite number; found {number_text}"
                )));
            };
            Ok(JsonValue::Number(json_number))
        }
        RuntimeValue::Bool(value) => Ok(JsonValue::Bool(*value)),
        RuntimeValue::Nil => Ok(JsonValue::Null),
        RuntimeValue::String(text) | RuntimeValue::Atom(text) => {
            Ok(JsonValue::String(text.clone()))
        }
        RuntimeValue::ResultOk(inner) => {
            let mut object = JsonMap::new();
            object.insert(
                "ok".to_string(),
                runtime_value_to_json(function, &log_field_path(path, "ok"), inner)?,
            );
            Ok(JsonValue::Object(object))
        }
        RuntimeValue::ResultErr(inner) => {
            let mut object = JsonMap::new();
            object.insert(
                "err".to_string(),
                runtime_value_to_json(function, &log_field_path(path, "err"), inner)?,
            );
            Ok(JsonValue::Object(object))
        }
        RuntimeValue::Tuple(left, right) => Ok(JsonValue::Array(vec![
            runtime_value_to_json(function, &log_field_path(path, "0"), left)?,
            runtime_value_to_json(function, &log_field_path(path, "1"), right)?,
        ])),
        RuntimeValue::Map(entries) | RuntimeValue::Keyword(entries) => Ok(JsonValue::Object(
            runtime_entries_to_json_object(function, path, entries)?,
        )),
        RuntimeValue::List(items) => {
            let mut json_items = Vec::with_capacity(items.len());
            for (index, item) in items.iter().enumerate() {
                json_items.push(runtime_value_to_json(
                    function,
                    &log_field_path(path, &index.to_string()),
                    item,
                )?);
            }
            Ok(JsonValue::Array(json_items))
        }
        RuntimeValue::Range(start, end) => {
            let mut object = JsonMap::new();
            object.insert("start".to_string(), JsonValue::Number((*start).into()));
            object.insert("end".to_string(), JsonValue::Number((*end).into()));
            Ok(JsonValue::Object(object))
        }
        RuntimeValue::SteppedRange(start, end, step) => {
            let mut object = JsonMap::new();
            object.insert("start".to_string(), JsonValue::Number((*start).into()));
            object.insert("end".to_string(), JsonValue::Number((*end).into()));
            object.insert("step".to_string(), JsonValue::Number((*step).into()));
            Ok(JsonValue::Object(object))
        }
        RuntimeValue::Closure(_) => Err(HostError::new(format!(
            "{function} {path} does not support function values"
        ))),
    }
}

fn resolve_structured_log_sink_path() -> Option<PathBuf> {
    let configured = std::env::var(STRUCTURED_LOG_PATH_ENV).ok()?;
    let trimmed = configured.trim();

    if trimmed.is_empty() {
        None
    } else {
        Some(PathBuf::from(trimmed))
    }
}

fn append_structured_log_line(serialized: &str) -> Result<(), HostError> {
    if let Some(path) = resolve_structured_log_sink_path() {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|error| {
                    HostError::new(format!(
                        "sys_log failed to create sink directory '{}': {error}",
                        parent.display()
                    ))
                })?;
            }
        }

        let mut sink = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| {
                HostError::new(format!(
                    "sys_log failed to open sink '{}': {error}",
                    path.display()
                ))
            })?;

        sink.write_all(serialized.as_bytes()).map_err(|error| {
            HostError::new(format!(
                "sys_log failed to append sink '{}': {error}",
                path.display()
            ))
        })?;
        sink.write_all(b"\n").map_err(|error| {
            HostError::new(format!(
                "sys_log failed to append sink '{}': {error}",
                path.display()
            ))
        })?;

        return Ok(());
    }

    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "{serialized}")
        .map_err(|error| HostError::new(format!("sys_log failed to write stderr sink: {error}")))
}

fn unix_timestamp_ms() -> i64 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_millis().min(i64::MAX as u128) as i64,
        Err(_) => 0,
    }
}

pub(super) fn atom_key(key: &str) -> RuntimeValue {
    RuntimeValue::Atom(key.to_string())
}

pub(super) fn map_with_atom_keys(entries: Vec<(&str, RuntimeValue)>) -> RuntimeValue {
    RuntimeValue::Map(
        entries
            .into_iter()
            .map(|(key, value)| (atom_key(key), value))
            .collect(),
    )
}

pub(super) fn tuple_string_pair(left: String, right: String) -> RuntimeValue {
    RuntimeValue::Tuple(
        Box::new(RuntimeValue::String(left)),
        Box::new(RuntimeValue::String(right)),
    )
}

fn is_executable_file(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        true
    }
}

fn find_command_on_path(name: &str) -> Option<PathBuf> {
    let candidate = Path::new(name);
    if candidate.components().count() > 1 {
        return is_executable_file(candidate).then(|| candidate.to_path_buf());
    }

    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        let direct = dir.join(name);
        if is_executable_file(&direct) {
            return Some(direct);
        }

        if cfg!(windows) {
            for ext in ["exe", "cmd", "bat"] {
                let with_ext = dir.join(format!("{name}.{ext}"));
                if is_executable_file(&with_ext) {
                    return Some(with_ext);
                }
            }
        }
    }

    None
}

fn parse_http_method(value: &str) -> Result<Method, HostError> {
    let upper = value.to_ascii_uppercase();
    if !matches!(
        upper.as_str(),
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD"
    ) {
        return Err(HostError::new(format!(
            "sys_http_request invalid method: {value}"
        )));
    }

    Method::from_bytes(upper.as_bytes())
        .map_err(|_| HostError::new(format!("sys_http_request invalid method: {value}")))
}

fn parse_http_headers(items: &[RuntimeValue]) -> Result<Vec<(String, String)>, HostError> {
    let mut headers = Vec::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let RuntimeValue::Tuple(name_value, header_value) = item else {
            return Err(HostError::new(format!(
                "sys_http_request headers argument 3 entry {} must be {{string, string}}; found {}",
                index + 1,
                host_value_kind(item)
            )));
        };

        let RuntimeValue::String(name) = name_value.as_ref() else {
            return Err(HostError::new(format!(
                "sys_http_request headers argument 3 entry {} expects string header name; found {}",
                index + 1,
                host_value_kind(name_value.as_ref())
            )));
        };

        let RuntimeValue::String(value) = header_value.as_ref() else {
            return Err(HostError::new(format!(
                "sys_http_request headers argument 3 entry {} expects string header value; found {}",
                index + 1,
                host_value_kind(header_value.as_ref())
            )));
        };

        headers.push((name.clone(), value.clone()));
    }

    Ok(headers)
}

fn parse_http_opts(
    entries: &[(RuntimeValue, RuntimeValue)],
) -> Result<HttpRequestOptions, HostError> {
    let mut opts = HttpRequestOptions {
        timeout_ms: HTTP_TIMEOUT_DEFAULT_MS,
        max_response_bytes: HTTP_MAX_RESPONSE_DEFAULT_BYTES,
        follow_redirects: HTTP_FOLLOW_REDIRECTS_DEFAULT,
        max_redirects: HTTP_MAX_REDIRECTS_DEFAULT,
    };

    for (key, value) in entries {
        let RuntimeValue::Atom(name) = key else {
            return Err(HostError::new(format!(
                "sys_http_request opts expects atom keys; found {}",
                host_value_kind(key)
            )));
        };

        match name.as_str() {
            "timeout_ms" => {
                let RuntimeValue::Int(timeout_ms) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.timeout_ms expects int; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.timeout_ms = *timeout_ms;
            }
            "max_response_bytes" => {
                let RuntimeValue::Int(max_response_bytes) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.max_response_bytes expects int; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.max_response_bytes = *max_response_bytes;
            }
            "follow_redirects" => {
                let RuntimeValue::Bool(follow_redirects) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.follow_redirects expects bool; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.follow_redirects = *follow_redirects;
            }
            "max_redirects" => {
                let RuntimeValue::Int(max_redirects) = value else {
                    return Err(HostError::new(format!(
                        "sys_http_request opts.max_redirects expects int; found {}",
                        host_value_kind(value)
                    )));
                };
                opts.max_redirects = *max_redirects;
            }
            other => {
                return Err(HostError::new(format!(
                    "sys_http_request unsupported opts key: {other}"
                )));
            }
        }
    }

    if opts.timeout_ms < HTTP_TIMEOUT_MIN_MS || opts.timeout_ms > HTTP_TIMEOUT_MAX_MS {
        return Err(HostError::new(format!(
            "sys_http_request timeout_ms out of range: {}",
            opts.timeout_ms
        )));
    }

    if opts.max_response_bytes < 1 || opts.max_response_bytes > HTTP_MAX_RESPONSE_MAX_BYTES {
        return Err(HostError::new(format!(
            "sys_http_request max_response_bytes out of range: {}",
            opts.max_response_bytes
        )));
    }

    if opts.max_redirects < 0 || opts.max_redirects > HTTP_MAX_REDIRECTS_MAX {
        return Err(HostError::new(format!(
            "sys_http_request max_redirects out of range: {}",
            opts.max_redirects
        )));
    }

    Ok(opts)
}

fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn decode_fixed_hex<const N: usize>(
    function: &str,
    field_name: &str,
    hex_value: &str,
) -> Result<[u8; N], HostError> {
    let expected_len = N * 2;
    if hex_value.len() != expected_len {
        return Err(HostError::new(format!(
            "{function} {field_name} must be {expected_len} hex chars, found {}",
            hex_value.len()
        )));
    }

    let mut decoded = [0u8; N];
    let bytes = hex_value.as_bytes();

    for (index, decoded_byte) in decoded.iter_mut().enumerate() {
        let high_index = index * 2;
        let low_index = high_index + 1;

        let Some(high_nibble) = hex_nibble(bytes[high_index]) else {
            return Err(HostError::new(format!(
                "{function} {field_name} contains non-hex character at position {}",
                high_index + 1
            )));
        };
        let Some(low_nibble) = hex_nibble(bytes[low_index]) else {
            return Err(HostError::new(format!(
                "{function} {field_name} contains non-hex character at position {}",
                low_index + 1
            )));
        };

        *decoded_byte = (high_nibble << 4) | low_nibble;
    }

    Ok(decoded)
}

fn clamp_delay_ms(delay_ms: i64, max_delay_ms: i64) -> i64 {
    delay_ms.clamp(0, max_delay_ms)
}

fn parse_retry_after_delay_ms(value: &str, max_delay_ms: i64) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(seconds) = trimmed.parse::<i64>() {
        if seconds < 0 {
            return Some(0);
        }
        let delay_ms = seconds.saturating_mul(1_000);
        return Some(clamp_delay_ms(delay_ms, max_delay_ms));
    }

    if let Ok(retry_at) = httpdate::parse_http_date(trimmed) {
        let now = std::time::SystemTime::now();
        let delay_ms = match retry_at.duration_since(now) {
            Ok(duration) => {
                let millis = duration.as_millis();
                if millis > i64::MAX as u128 {
                    i64::MAX
                } else {
                    millis as i64
                }
            }
            Err(_) => 0,
        };
        return Some(clamp_delay_ms(delay_ms, max_delay_ms));
    }

    None
}

fn exponential_backoff_ms(attempt: i64, base_delay_ms: i64, max_delay_ms: i64) -> i64 {
    let exponent = attempt.saturating_sub(1).min(62) as u32;
    let multiplier = 1i64.checked_shl(exponent).unwrap_or(i64::MAX);
    let raw_delay_ms = base_delay_ms.saturating_mul(multiplier);
    clamp_delay_ms(raw_delay_ms, max_delay_ms)
}

fn deterministic_jitter_ms(attempt: i64, status_code: i64, max_jitter_ms: i64) -> i64 {
    if max_jitter_ms == 0 {
        return 0;
    }

    let seed = attempt
        .unsigned_abs()
        .wrapping_mul(1_103_515_245)
        .wrapping_add(status_code.unsigned_abs())
        .wrapping_add(12_345);

    (seed % (max_jitter_ms as u64 + 1)) as i64
}

fn retry_plan_result(retry: bool, delay_ms: i64, source: &str) -> RuntimeValue {
    map_with_atom_keys(vec![
        ("retry", RuntimeValue::Bool(retry)),
        ("delay_ms", RuntimeValue::Int(delay_ms)),
        ("source", RuntimeValue::Atom(source.to_string())),
    ])
}

fn host_sys_sleep_ms(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_sleep_ms", args, 1)?;
    let delay_ms = expect_int_arg("sys_sleep_ms", args, 0)?;

    if !(0..=SLEEP_MAX_MS).contains(&delay_ms) {
        return Err(HostError::new(format!(
            "sys_sleep_ms delay_ms out of range: {delay_ms}"
        )));
    }

    if delay_ms > 0 {
        std::thread::sleep(Duration::from_millis(delay_ms as u64));
    }

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_retry_plan(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_retry_plan", args, 7)?;

    let status_code = expect_int_arg("sys_retry_plan", args, 0)?;
    let attempt = expect_int_arg("sys_retry_plan", args, 1)?;
    let max_attempts = expect_int_arg("sys_retry_plan", args, 2)?;
    let base_delay_ms = expect_int_arg("sys_retry_plan", args, 3)?;
    let max_delay_ms = expect_int_arg("sys_retry_plan", args, 4)?;
    let jitter_ms = expect_int_arg("sys_retry_plan", args, 5)?;
    let retry_after = match args.get(6) {
        Some(RuntimeValue::String(value)) => value.clone(),
        Some(RuntimeValue::Nil) => String::new(),
        Some(other) => {
            return Err(HostError::new(format!(
                "sys_retry_plan expects string-or-nil argument 7; found {}",
                host_value_kind(other)
            )))
        }
        None => return Err(HostError::new("sys_retry_plan missing required argument 7")),
    };

    if !(RETRY_STATUS_MIN..=RETRY_STATUS_MAX).contains(&status_code) {
        return Err(HostError::new(format!(
            "sys_retry_plan status out of range: {status_code}"
        )));
    }

    if attempt < 1 {
        return Err(HostError::new(format!(
            "sys_retry_plan attempt must be >= 1, found {attempt}"
        )));
    }

    if !(1..=RETRY_MAX_ATTEMPTS_CAP).contains(&max_attempts) {
        return Err(HostError::new(format!(
            "sys_retry_plan max_attempts out of range: {max_attempts}"
        )));
    }

    if !(1..=RETRY_DELAY_MAX_MS).contains(&base_delay_ms) {
        return Err(HostError::new(format!(
            "sys_retry_plan base_delay_ms out of range: {base_delay_ms}"
        )));
    }

    if !(1..=RETRY_DELAY_MAX_MS).contains(&max_delay_ms) {
        return Err(HostError::new(format!(
            "sys_retry_plan max_delay_ms out of range: {max_delay_ms}"
        )));
    }

    if max_delay_ms < base_delay_ms {
        return Err(HostError::new(format!(
            "sys_retry_plan max_delay_ms must be >= base_delay_ms; found {} < {}",
            max_delay_ms, base_delay_ms
        )));
    }

    if !(0..=RETRY_JITTER_MAX_MS).contains(&jitter_ms) {
        return Err(HostError::new(format!(
            "sys_retry_plan jitter_ms out of range: {jitter_ms}"
        )));
    }

    if jitter_ms > max_delay_ms {
        return Err(HostError::new(format!(
            "sys_retry_plan jitter_ms must be <= max_delay_ms; found {} > {}",
            jitter_ms, max_delay_ms
        )));
    }

    if attempt >= max_attempts {
        return Ok(retry_plan_result(false, 0, "exhausted"));
    }

    let retryable_status = status_code == 429 || (500..=599).contains(&status_code);
    if !retryable_status {
        return Ok(retry_plan_result(false, 0, "non_retryable"));
    }

    if status_code == 429 {
        if let Some(delay_ms) = parse_retry_after_delay_ms(&retry_after, max_delay_ms) {
            return Ok(retry_plan_result(true, delay_ms, "retry_after"));
        }
    }

    let backoff_ms = exponential_backoff_ms(attempt, base_delay_ms, max_delay_ms);
    let jitter = deterministic_jitter_ms(attempt, status_code, jitter_ms);
    let bounded_delay_ms = clamp_delay_ms(backoff_ms.saturating_add(jitter), max_delay_ms);

    Ok(retry_plan_result(true, bounded_delay_ms, "backoff"))
}

fn host_sys_log(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_log", args, 3)?;

    let level_raw = expect_string_or_atom_arg("sys_log", args, 0)?;
    let Some(level) = log_level_label(&level_raw) else {
        return Err(HostError::new(format!(
            "sys_log level must be one of debug|info|warn|error; found {level_raw}"
        )));
    };

    let event = expect_string_or_atom_arg("sys_log", args, 1)?;
    if event.trim().is_empty() {
        return Err(HostError::new("sys_log event must not be empty"));
    }

    let fields = expect_map_arg("sys_log", args, 2)?;
    let fields_json = runtime_entries_to_json_object("sys_log", "fields", &fields)?;

    let mut payload = JsonMap::new();
    payload.insert(
        "timestamp_ms".to_string(),
        JsonValue::Number(JsonNumber::from(unix_timestamp_ms())),
    );
    payload.insert("level".to_string(), JsonValue::String(level.to_string()));
    payload.insert("event".to_string(), JsonValue::String(event));
    payload.insert("fields".to_string(), JsonValue::Object(fields_json));

    let serialized = serde_json::to_string(&JsonValue::Object(payload))
        .map_err(|error| HostError::new(format!("sys_log failed to serialize payload: {error}")))?;

    append_structured_log_line(&serialized)?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_run(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_run", args, 1)?;
    let command = expect_string_arg("sys_run", args, 0)?;
    let shell_command = format!("{command} 2>&1");

    let output = std::process::Command::new("sh")
        .args(["-lc", &shell_command])
        .output()
        .map_err(|error| {
            HostError::new(format!("sys_run failed to execute shell command: {error}"))
        })?;

    let exit_code = output.status.code().unwrap_or(-1);
    let combined_output = String::from_utf8_lossy(&output.stdout).into_owned();

    Ok(map_with_atom_keys(vec![
        ("exit_code", RuntimeValue::Int(exit_code as i64)),
        ("output", RuntimeValue::String(combined_output)),
    ]))
}

fn host_sys_path_exists(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_path_exists", args, 1)?;
    let path = expect_string_arg("sys_path_exists", args, 0)?;
    Ok(RuntimeValue::Bool(Path::new(&path).exists()))
}

fn collect_relative_files_recursive(
    root_path: &Path,
    current_path: &Path,
    files: &mut Vec<String>,
) -> Result<(), HostError> {
    let mut entries = std::fs::read_dir(current_path)
        .map_err(|error| {
            HostError::new(format!(
                "sys_list_files_recursive failed for '{}': {error}",
                root_path.display()
            ))
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            HostError::new(format!(
                "sys_list_files_recursive failed for '{}': {error}",
                root_path.display()
            ))
        })?;

    entries.sort_by(|left, right| {
        left.file_name()
            .to_string_lossy()
            .cmp(&right.file_name().to_string_lossy())
    });

    for entry in entries {
        let entry_path = entry.path();
        let file_type = entry.file_type().map_err(|error| {
            HostError::new(format!(
                "sys_list_files_recursive failed for '{}': {error}",
                root_path.display()
            ))
        })?;

        if file_type.is_dir() {
            collect_relative_files_recursive(root_path, &entry_path, files)?;
        } else if file_type.is_file() {
            let relative_path = entry_path
                .strip_prefix(root_path)
                .map_err(|error| {
                    HostError::new(format!(
                        "sys_list_files_recursive failed for '{}': {error}",
                        root_path.display()
                    ))
                })?
                .to_string_lossy()
                .replace('\\', "/");
            files.push(relative_path);
        }
    }

    Ok(())
}

fn host_sys_list_files_recursive(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_list_files_recursive", args, 1)?;
    let path = expect_string_arg("sys_list_files_recursive", args, 0)?;

    if path.is_empty() {
        return Err(HostError::new(
            "sys_list_files_recursive path must not be empty",
        ));
    }

    let root_path = Path::new(&path);
    let mut files = Vec::new();

    collect_relative_files_recursive(root_path, root_path, &mut files)?;

    Ok(RuntimeValue::List(
        files.into_iter().map(RuntimeValue::String).collect(),
    ))
}

fn host_sys_ensure_dir(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_ensure_dir", args, 1)?;
    let path = expect_string_arg("sys_ensure_dir", args, 0)?;

    std::fs::create_dir_all(&path).map_err(|error| {
        HostError::new(format!("sys_ensure_dir failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn remove_tree(path: &Path, display_path: &str) -> Result<bool, HostError> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(HostError::new(format!(
                "sys_remove_tree failed for '{}': {error}",
                display_path
            )))
        }
    };

    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path).map_err(|error| {
            HostError::new(format!(
                "sys_remove_tree failed for '{}': {error}",
                display_path
            ))
        })?;
    } else {
        std::fs::remove_file(path).map_err(|error| {
            HostError::new(format!(
                "sys_remove_tree failed for '{}': {error}",
                display_path
            ))
        })?;
    }

    Ok(true)
}

fn host_sys_remove_tree(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_remove_tree", args, 1)?;
    let path = expect_string_arg("sys_remove_tree", args, 0)?;

    if path.is_empty() {
        return Err(HostError::new("sys_remove_tree path must not be empty"));
    }

    Ok(RuntimeValue::Bool(remove_tree(Path::new(&path), &path)?))
}

fn ensure_parent_directory(function: &str, path: &Path) -> Result<(), HostError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|error| {
                HostError::new(format!(
                    "{function} failed to create parent directory '{}': {error}",
                    parent.display()
                ))
            })?;
        }
    }

    Ok(())
}

fn atomic_temp_path(target: &Path) -> PathBuf {
    let timestamp_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let base_name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let temp_name = format!(".{base_name}.tmp.{pid}.{timestamp_nanos}");

    if let Some(parent) = target.parent() {
        if parent.as_os_str().is_empty() {
            PathBuf::from(temp_name)
        } else {
            parent.join(temp_name)
        }
    } else {
        PathBuf::from(temp_name)
    }
}

fn write_text_atomic(function: &str, path: &str, content: &str) -> Result<(), HostError> {
    let target = Path::new(path);
    ensure_parent_directory(function, target)?;

    let temp_path = atomic_temp_path(target);
    let write_result = (|| -> Result<(), std::io::Error> {
        let mut temp_file = std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)?;

        temp_file.write_all(content.as_bytes())?;
        temp_file.sync_all()?;
        std::fs::rename(&temp_path, target)?;
        Ok(())
    })();

    match write_result {
        Ok(()) => Ok(()),
        Err(error) => {
            let _ = std::fs::remove_file(&temp_path);
            Err(HostError::new(format!(
                "{function} failed for '{}': {error}",
                path
            )))
        }
    }
}

fn host_sys_write_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_write_text", args, 2)?;
    let path = expect_string_arg("sys_write_text", args, 0)?;
    let content = expect_string_arg("sys_write_text", args, 1)?;

    std::fs::write(&path, content).map_err(|error| {
        HostError::new(format!("sys_write_text failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_append_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_append_text", args, 2)?;
    let path = expect_string_arg("sys_append_text", args, 0)?;
    let content = expect_string_arg("sys_append_text", args, 1)?;
    let target = Path::new(&path);

    ensure_parent_directory("sys_append_text", target)?;

    let mut sink = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(target)
        .map_err(|error| {
            HostError::new(format!("sys_append_text failed for '{}': {error}", path))
        })?;

    sink.write_all(content.as_bytes()).map_err(|error| {
        HostError::new(format!("sys_append_text failed for '{}': {error}", path))
    })?;
    sink.sync_data().map_err(|error| {
        HostError::new(format!("sys_append_text failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_write_text_atomic(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_write_text_atomic", args, 2)?;
    let path = expect_string_arg("sys_write_text_atomic", args, 0)?;
    let content = expect_string_arg("sys_write_text_atomic", args, 1)?;

    write_text_atomic("sys_write_text_atomic", &path, &content)?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_lock_acquire(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_lock_acquire", args, 1)?;
    let path = expect_string_arg("sys_lock_acquire", args, 0)?;
    let target = Path::new(&path);

    ensure_parent_directory("sys_lock_acquire", target)?;

    let lock_attempt = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(target);

    let mut handle = match lock_attempt {
        Ok(handle) => handle,
        Err(error) if error.kind() == ErrorKind::AlreadyExists => {
            return Ok(RuntimeValue::Bool(false));
        }
        Err(error) => {
            return Err(HostError::new(format!(
                "sys_lock_acquire failed for '{}': {error}",
                path
            )));
        }
    };

    let marker = format!(
        "pid={} timestamp_ms={}\n",
        std::process::id(),
        unix_timestamp_ms()
    );

    handle.write_all(marker.as_bytes()).map_err(|error| {
        HostError::new(format!("sys_lock_acquire failed for '{}': {error}", path))
    })?;
    handle.sync_all().map_err(|error| {
        HostError::new(format!("sys_lock_acquire failed for '{}': {error}", path))
    })?;

    Ok(RuntimeValue::Bool(true))
}

fn host_sys_lock_release(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_lock_release", args, 1)?;
    let path = expect_string_arg("sys_lock_release", args, 0)?;

    match std::fs::remove_file(&path) {
        Ok(()) => Ok(RuntimeValue::Bool(true)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(RuntimeValue::Bool(false)),
        Err(error) => Err(HostError::new(format!(
            "sys_lock_release failed for '{}': {error}",
            path
        ))),
    }
}

fn host_sys_read_text(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_read_text", args, 1)?;
    let path = expect_string_arg("sys_read_text", args, 0)?;

    let content = std::fs::read_to_string(&path)
        .map_err(|error| HostError::new(format!("sys_read_text failed for '{}': {error}", path)))?;

    Ok(RuntimeValue::String(content))
}

fn host_sys_read_stdin(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_read_stdin", args, 0)?;

    let mut buffer = Vec::new();
    std::io::stdin()
        .read_to_end(&mut buffer)
        .map_err(|error| HostError::new(format!("sys_read_stdin failed: {error}")))?;

    Ok(RuntimeValue::String(
        String::from_utf8_lossy(&buffer).into_owned(),
    ))
}

fn host_sys_http_request(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_http_request", args, 5)?;

    let method_value = expect_string_arg("sys_http_request", args, 0)?;
    let method = parse_http_method(&method_value)?;

    let url_value = expect_string_arg("sys_http_request", args, 1)?;
    let url = reqwest::Url::parse(&url_value)
        .map_err(|_| HostError::new(format!("sys_http_request invalid url: {url_value}")))?;

    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(HostError::new(format!(
                "sys_http_request unsupported url scheme: {other}"
            )));
        }
    }

    let headers_value = expect_list_arg("sys_http_request", args, 2)?;
    let headers = parse_http_headers(&headers_value)?;

    let body = expect_string_arg("sys_http_request", args, 3)?;

    let opts_value = expect_map_arg("sys_http_request", args, 4)?;
    let opts = parse_http_opts(&opts_value)?;

    let redirect_policy = if opts.follow_redirects {
        reqwest::redirect::Policy::limited(opts.max_redirects as usize)
    } else {
        reqwest::redirect::Policy::none()
    };

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(opts.timeout_ms as u64))
        .redirect(redirect_policy)
        .build()
        .map_err(|error| HostError::new(format!("sys_http_request failed: {error}")))?;

    let mut request = client.request(method, url.clone());
    for (name, value) in headers {
        request = request.header(name, value);
    }

    let response = request
        .body(body)
        .send()
        .map_err(|error| HostError::new(format!("sys_http_request failed: {error}")))?;

    let status = response.status().as_u16() as i64;
    let final_url = response.url().to_string();

    let mut response_headers = response
        .headers()
        .iter()
        .map(|(name, value)| {
            let rendered_value = value.to_str().unwrap_or_default().to_string();
            (name.as_str().to_ascii_lowercase(), rendered_value)
        })
        .collect::<Vec<_>>();
    response_headers.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

    let mut body_bytes = Vec::new();
    let mut limited_reader = response.take((opts.max_response_bytes as u64) + 1);
    limited_reader
        .read_to_end(&mut body_bytes)
        .map_err(|error| HostError::new(format!("sys_http_request failed: {error}")))?;

    if body_bytes.len() > opts.max_response_bytes as usize {
        return Err(HostError::new(format!(
            "sys_http_request response exceeded max_response_bytes: {}",
            opts.max_response_bytes
        )));
    }

    let response_body = String::from_utf8_lossy(&body_bytes).into_owned();

    Ok(map_with_atom_keys(vec![
        ("status", RuntimeValue::Int(status)),
        (
            "headers",
            RuntimeValue::List(
                response_headers
                    .into_iter()
                    .map(|(name, value)| tuple_string_pair(name, value))
                    .collect(),
            ),
        ),
        ("body", RuntimeValue::String(response_body)),
        ("final_url", RuntimeValue::String(final_url)),
    ]))
}

fn host_sys_env(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_env", args, 1)?;
    let key = expect_string_arg("sys_env", args, 0)?;

    let value = std::env::var_os(&key)
        .map(|v| RuntimeValue::String(v.to_string_lossy().into_owned()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

fn host_sys_which(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_which", args, 1)?;
    let command = expect_string_arg("sys_which", args, 0)?;

    let value = find_command_on_path(&command)
        .map(|path| RuntimeValue::String(path.display().to_string()))
        .unwrap_or(RuntimeValue::Nil);

    Ok(value)
}

fn host_sys_cwd(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_cwd", args, 0)?;

    let cwd = std::env::current_dir()
        .map_err(|error| HostError::new(format!("sys_cwd failed to read current dir: {error}")))?;

    Ok(RuntimeValue::String(cwd.display().to_string()))
}

fn host_sys_argv(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_argv", args, 0)?;

    let argv_list = std::env::args()
        .map(RuntimeValue::String)
        .collect::<Vec<_>>();

    Ok(RuntimeValue::List(argv_list))
}

fn host_sys_constant_time_eq(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_constant_time_eq", args, 2)?;
    let left = expect_string_arg("sys_constant_time_eq", args, 0)?;
    let right = expect_string_arg("sys_constant_time_eq", args, 1)?;

    let equal = left.as_bytes().ct_eq(right.as_bytes()).unwrap_u8() == 1;
    Ok(RuntimeValue::Bool(equal))
}

fn host_sys_discord_ed25519_verify(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_discord_ed25519_verify", args, 4)?;
    let public_key_hex = expect_string_arg("sys_discord_ed25519_verify", args, 0)?;
    let signature_hex = expect_string_arg("sys_discord_ed25519_verify", args, 1)?;
    let timestamp = expect_string_arg("sys_discord_ed25519_verify", args, 2)?;
    let body = expect_string_arg("sys_discord_ed25519_verify", args, 3)?;

    let public_key_bytes = decode_fixed_hex::<ED25519_PUBLIC_KEY_BYTES>(
        "sys_discord_ed25519_verify",
        "public_key_hex",
        &public_key_hex,
    )?;
    let signature_bytes = decode_fixed_hex::<ED25519_SIGNATURE_BYTES>(
        "sys_discord_ed25519_verify",
        "signature_hex",
        &signature_hex,
    )?;

    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes).map_err(|error| {
        HostError::new(format!(
            "sys_discord_ed25519_verify invalid public_key_hex bytes: {error}"
        ))
    })?;

    let signature = Signature::from_bytes(&signature_bytes);

    let mut signed_payload = String::with_capacity(timestamp.len() + body.len());
    signed_payload.push_str(&timestamp);
    signed_payload.push_str(&body);

    let is_valid = verifying_key
        .verify(signed_payload.as_bytes(), &signature)
        .is_ok();

    Ok(RuntimeValue::Bool(is_valid))
}

fn host_sys_random_token(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_random_token", args, 1)?;
    let bytes = expect_int_arg("sys_random_token", args, 0)?;

    if !(RANDOM_TOKEN_MIN_BYTES..=RANDOM_TOKEN_MAX_BYTES).contains(&bytes) {
        return Err(HostError::new(format!(
            "sys_random_token bytes out of range: {bytes}"
        )));
    }

    let mut buffer = vec![0u8; bytes as usize];
    rand::rng().fill_bytes(&mut buffer);

    Ok(RuntimeValue::String(URL_SAFE_NO_PAD.encode(&buffer)))
}

fn host_sys_hmac_sha256_hex(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_hmac_sha256_hex", args, 2)?;
    let secret = expect_string_arg("sys_hmac_sha256_hex", args, 0)?;
    let message = expect_string_arg("sys_hmac_sha256_hex", args, 1)?;

    if secret.is_empty() {
        return Err(HostError::new(
            "sys_hmac_sha256_hex secret must not be empty",
        ));
    }

    if message.is_empty() {
        return Err(HostError::new(
            "sys_hmac_sha256_hex message must not be empty",
        ));
    }

    type HmacSha256 = Hmac<Sha256>;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|e| HostError::new(format!("sys_hmac_sha256_hex failed: {e}")))?;
    mac.update(message.as_bytes());

    let result = mac.finalize();
    let hex: String = result
        .into_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();

    Ok(RuntimeValue::String(hex))
}

pub(super) fn register_system_host_functions(registry: &HostRegistry) {
    registry.register("sys_run", host_sys_run);
    registry.register("sys_sleep_ms", host_sys_sleep_ms);
    registry.register("sys_retry_plan", host_sys_retry_plan);
    registry.register("sys_log", host_sys_log);
    registry.register("sys_path_exists", host_sys_path_exists);
    registry.register("sys_list_files_recursive", host_sys_list_files_recursive);
    registry.register("sys_ensure_dir", host_sys_ensure_dir);
    registry.register("sys_remove_tree", host_sys_remove_tree);
    registry.register("sys_write_text", host_sys_write_text);
    registry.register("sys_append_text", host_sys_append_text);
    registry.register("sys_write_text_atomic", host_sys_write_text_atomic);
    registry.register("sys_lock_acquire", host_sys_lock_acquire);
    registry.register("sys_lock_release", host_sys_lock_release);
    registry.register("sys_read_text", host_sys_read_text);
    registry.register("sys_read_stdin", host_sys_read_stdin);
    registry.register("sys_http_request", host_sys_http_request);
    registry.register("sys_env", host_sys_env);
    registry.register("sys_which", host_sys_which);
    registry.register("sys_cwd", host_sys_cwd);
    registry.register("sys_argv", host_sys_argv);
    registry.register("sys_constant_time_eq", host_sys_constant_time_eq);
    registry.register(
        "sys_discord_ed25519_verify",
        host_sys_discord_ed25519_verify,
    );
    registry.register("sys_random_token", host_sys_random_token);
    registry.register("sys_hmac_sha256_hex", host_sys_hmac_sha256_hex);
}
