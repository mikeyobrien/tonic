use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use std::time::{SystemTime, UNIX_EPOCH};
use time::OffsetDateTime;

fn expect_exact_args(
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

/// Returns the current UTC time as an ISO 8601 string (e.g. "2026-03-26T12:34:56Z").
fn datetime_utc_now(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("DateTime.utc_now", args, 0)?;
    let now = OffsetDateTime::now_utc();
    let formatted = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
    );
    Ok(RuntimeValue::String(formatted))
}

/// Returns the current Unix timestamp in seconds as an integer.
fn datetime_unix_now(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("DateTime.unix_now", args, 0)?;
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| HostError::new(format!("DateTime.unix_now: {e}")))?
        .as_secs();
    Ok(RuntimeValue::Int(secs as i64))
}

/// Returns the current Unix timestamp in milliseconds as an integer.
fn datetime_unix_now_ms(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("DateTime.unix_now_ms", args, 0)?;
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| HostError::new(format!("DateTime.unix_now_ms: {e}")))?
        .as_millis();
    Ok(RuntimeValue::Int(millis as i64))
}

pub(crate) fn register_datetime_host_functions(registry: &HostRegistry) {
    registry.register("datetime_utc_now", datetime_utc_now);
    registry.register("datetime_unix_now", datetime_unix_now);
    registry.register("datetime_unix_now_ms", datetime_unix_now_ms);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_now_returns_iso8601_string() {
        let result = datetime_utc_now(&[]).unwrap();
        match &result {
            RuntimeValue::String(s) => {
                assert!(s.ends_with('Z'), "should end with Z: {s}");
                assert_eq!(s.len(), 20, "ISO 8601 format YYYY-MM-DDTHH:MM:SSZ: {s}");
                assert_eq!(&s[4..5], "-");
                assert_eq!(&s[7..8], "-");
                assert_eq!(&s[10..11], "T");
                assert_eq!(&s[13..14], ":");
                assert_eq!(&s[16..17], ":");
            }
            other => panic!("expected string, got {other:?}"),
        }
    }

    #[test]
    fn utc_now_rejects_arguments() {
        let err = datetime_utc_now(&[RuntimeValue::Int(1)]).unwrap_err();
        assert!(err.message.contains("expects exactly 0"));
    }

    #[test]
    fn unix_now_returns_reasonable_timestamp() {
        let result = datetime_unix_now(&[]).unwrap();
        match result {
            RuntimeValue::Int(secs) => {
                // Should be after 2024-01-01 (1704067200) and before 2100
                assert!(secs > 1_704_067_200, "timestamp too small: {secs}");
                assert!(secs < 4_102_444_800, "timestamp too large: {secs}");
            }
            other => panic!("expected int, got {other:?}"),
        }
    }

    #[test]
    fn unix_now_rejects_arguments() {
        let err = datetime_unix_now(&[RuntimeValue::Int(1)]).unwrap_err();
        assert!(err.message.contains("expects exactly 0"));
    }

    #[test]
    fn unix_now_ms_returns_milliseconds() {
        let result = datetime_unix_now_ms(&[]).unwrap();
        match result {
            RuntimeValue::Int(ms) => {
                // ms should be roughly 1000x the seconds value
                assert!(ms > 1_704_067_200_000, "ms timestamp too small: {ms}");
            }
            other => panic!("expected int, got {other:?}"),
        }
    }

    #[test]
    fn unix_now_ms_greater_than_unix_now() {
        let secs = match datetime_unix_now(&[]).unwrap() {
            RuntimeValue::Int(s) => s,
            _ => panic!("expected int"),
        };
        let ms = match datetime_unix_now_ms(&[]).unwrap() {
            RuntimeValue::Int(m) => m,
            _ => panic!("expected int"),
        };
        assert!(
            ms >= secs * 1000,
            "ms ({ms}) should be >= secs*1000 ({secs})"
        );
    }

    #[test]
    fn unix_now_ms_rejects_arguments() {
        let err = datetime_unix_now_ms(&[RuntimeValue::Int(1)]).unwrap_err();
        assert!(err.message.contains("expects exactly 0"));
    }

    #[test]
    fn register_adds_all_functions() {
        let registry = HostRegistry::new();
        // Verify our functions are callable via the registry
        let result = registry.call("datetime_utc_now", &[]);
        assert!(result.is_ok(), "datetime_utc_now should be registered");
        let result = registry.call("datetime_unix_now", &[]);
        assert!(result.is_ok(), "datetime_unix_now should be registered");
        let result = registry.call("datetime_unix_now_ms", &[]);
        assert!(result.is_ok(), "datetime_unix_now_ms should be registered");
    }
}
