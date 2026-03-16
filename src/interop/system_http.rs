use super::{atom_key, expect_exact_args, expect_int_arg, expect_list_arg, expect_string_arg, map_with_atom_keys};
use super::super::HostError;
use crate::runtime::RuntimeValue;
use reqwest::blocking::Client;
use reqwest::Method;
use std::path::Path;
use std::time::Duration;

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

