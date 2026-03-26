use super::{host_value_kind, read_host_stdin_to_end, write_host_stderr, HostError, HostRegistry};
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
        RuntimeValue::List(items) | RuntimeValue::Binary(items) => {
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

    write_host_stderr(&format!("{serialized}\n"))?;
    Ok(())
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

#[path = "system_net.rs"]
mod net;
use net::*;

#[path = "system_io.rs"]
mod io;
use io::*;

pub(super) fn register_system_host_functions(registry: &HostRegistry) {
    registry.register("sys_run", host_sys_run);
    registry.register("sys_sleep_ms", host_sys_sleep_ms);
    registry.register("sys_retry_plan", host_sys_retry_plan);
    registry.register("sys_log", host_sys_log);
    registry.register("sys_path_exists", host_sys_path_exists);
    registry.register("sys_list_dir", host_sys_list_dir);
    registry.register("sys_is_dir", host_sys_is_dir);
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
