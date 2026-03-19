use super::*;

pub(super) fn parse_http_method(value: &str) -> Result<Method, HostError> {
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

pub(super) fn parse_http_headers(
    items: &[RuntimeValue],
) -> Result<Vec<(String, String)>, HostError> {
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

pub(super) fn parse_http_opts(
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

pub(super) fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(super) fn decode_fixed_hex<const N: usize>(
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

pub(super) fn host_sys_sleep_ms(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
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

pub(super) fn host_sys_retry_plan(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
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

pub(super) fn host_sys_log(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
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

pub(super) fn host_sys_run(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
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

pub(super) fn host_sys_path_exists(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_path_exists", args, 1)?;
    let path = expect_string_arg("sys_path_exists", args, 0)?;
    Ok(RuntimeValue::Bool(Path::new(&path).exists()))
}

pub(super) fn host_sys_list_dir(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_list_dir", args, 1)?;
    let path = expect_string_arg("sys_list_dir", args, 0)?;

    if path.is_empty() {
        return Err(HostError::new("sys_list_dir path must not be empty"));
    }

    let mut entries = std::fs::read_dir(&path)
        .map_err(|error| HostError::new(format!("sys_list_dir failed for '{}': {error}", path)))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| HostError::new(format!("sys_list_dir failed for '{}': {error}", path)))?;

    entries.sort_by(|left, right| {
        left.file_name()
            .to_string_lossy()
            .cmp(&right.file_name().to_string_lossy())
    });

    let names: Vec<RuntimeValue> = entries
        .into_iter()
        .map(|entry| RuntimeValue::String(entry.file_name().to_string_lossy().into_owned()))
        .collect();

    Ok(RuntimeValue::List(names))
}

pub(super) fn host_sys_is_dir(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("sys_is_dir", args, 1)?;
    let path = expect_string_arg("sys_is_dir", args, 0)?;
    Ok(RuntimeValue::Bool(Path::new(&path).is_dir()))
}

pub(super) fn collect_relative_files_recursive(
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
