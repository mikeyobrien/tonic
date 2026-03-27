use std::sync::atomic::{AtomicU8, Ordering};

use super::system::expect_exact_args;
use super::{host_value_kind, write_host_stream, HostError, HostOutputStream, HostRegistry};
use crate::runtime::RuntimeValue;

/// Log levels as u8 for atomic storage.
/// Higher value = more severe. Messages below the current level are suppressed.
const LEVEL_DEBUG: u8 = 0;
const LEVEL_INFO: u8 = 1;
const LEVEL_WARN: u8 = 2;
const LEVEL_ERROR: u8 = 3;
const LEVEL_NONE: u8 = 4;

static LOG_LEVEL: AtomicU8 = AtomicU8::new(LEVEL_INFO);

fn level_from_atom(s: &str) -> Result<u8, HostError> {
    match s {
        "debug" => Ok(LEVEL_DEBUG),
        "info" => Ok(LEVEL_INFO),
        "warn" => Ok(LEVEL_WARN),
        "error" => Ok(LEVEL_ERROR),
        "none" => Ok(LEVEL_NONE),
        other => Err(HostError::new(format!(
            "Logger.set_level expects :debug, :info, :warn, :error, or :none, got :{}",
            other
        ))),
    }
}

fn level_to_atom(level: u8) -> &'static str {
    match level {
        LEVEL_DEBUG => "debug",
        LEVEL_INFO => "info",
        LEVEL_WARN => "warn",
        LEVEL_ERROR => "error",
        _ => "none",
    }
}

fn log_at_level(
    func_name: &str,
    level: u8,
    label: &str,
    args: &[RuntimeValue],
) -> Result<RuntimeValue, HostError> {
    expect_exact_args(func_name, args, 1)?;
    let msg = match &args[0] {
        RuntimeValue::String(s) => s.clone(),
        other => {
            return Err(HostError::new(format!(
                "{} expects a string argument, found {}",
                func_name,
                host_value_kind(other)
            )));
        }
    };

    let current = LOG_LEVEL.load(Ordering::Relaxed);
    if level >= current {
        let line = format!("[{}] {}\n", label, msg);
        write_host_stream(HostOutputStream::Stderr, &line)?;
    }

    Ok(RuntimeValue::Atom("ok".into()))
}

fn host_logger_debug(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    log_at_level("Logger.debug", LEVEL_DEBUG, "debug", args)
}

fn host_logger_info(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    log_at_level("Logger.info", LEVEL_INFO, "info", args)
}

fn host_logger_warn(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    log_at_level("Logger.warn", LEVEL_WARN, "warn", args)
}

fn host_logger_error(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    log_at_level("Logger.error", LEVEL_ERROR, "error", args)
}

fn host_logger_set_level(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Logger.set_level", args, 1)?;
    let level_str = match &args[0] {
        RuntimeValue::Atom(s) => s.clone(),
        other => {
            return Err(HostError::new(format!(
                "Logger.set_level expects an atom (:debug, :info, :warn, :error, :none), found {}",
                host_value_kind(other)
            )));
        }
    };
    let level = level_from_atom(&level_str)?;
    LOG_LEVEL.store(level, Ordering::Relaxed);
    Ok(RuntimeValue::Atom("ok".into()))
}

fn host_logger_get_level(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Logger.get_level", args, 0)?;
    let current = LOG_LEVEL.load(Ordering::Relaxed);
    Ok(RuntimeValue::Atom(level_to_atom(current).into()))
}

pub fn register_logger_host_functions(registry: &HostRegistry) {
    registry.register("logger_debug", host_logger_debug);
    registry.register("logger_info", host_logger_info);
    registry.register("logger_warn", host_logger_warn);
    registry.register("logger_error", host_logger_error);
    registry.register("logger_set_level", host_logger_set_level);
    registry.register("logger_get_level", host_logger_get_level);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::capture_host_output_with_stdin;
    use std::sync::Mutex;

    // Serialize all logger tests — LOG_LEVEL is global shared state.
    static TEST_LOCK: Mutex<()> = Mutex::new(());

    fn rv_str(s: &str) -> RuntimeValue {
        RuntimeValue::String(s.into())
    }

    fn rv_atom(s: &str) -> RuntimeValue {
        RuntimeValue::Atom(s.into())
    }

    fn reset_level() {
        LOG_LEVEL.store(LEVEL_INFO, Ordering::Relaxed);
    }

    #[test]
    fn set_and_get_level_round_trip() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_level();
        assert_eq!(host_logger_get_level(&[]).unwrap(), rv_atom("info"));
        host_logger_set_level(&[rv_atom("debug")]).unwrap();
        assert_eq!(host_logger_get_level(&[]).unwrap(), rv_atom("debug"));
        host_logger_set_level(&[rv_atom("error")]).unwrap();
        assert_eq!(host_logger_get_level(&[]).unwrap(), rv_atom("error"));
        reset_level();
    }

    #[test]
    fn set_level_rejects_invalid_atom() {
        let result = host_logger_set_level(&[rv_atom("verbose")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(":verbose"));
    }

    #[test]
    fn set_level_rejects_non_atom() {
        let result = host_logger_set_level(&[rv_str("info")]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("atom"));
    }

    #[test]
    fn debug_suppressed_at_info_level() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_level();
        let (result, output) =
            capture_host_output_with_stdin(None, || host_logger_debug(&[rv_str("hidden")]));
        assert_eq!(result.unwrap(), rv_atom("ok"));
        assert_eq!(output.stderr, "");
    }

    #[test]
    fn debug_shown_at_debug_level() {
        let _g = TEST_LOCK.lock().unwrap();
        LOG_LEVEL.store(LEVEL_DEBUG, Ordering::Relaxed);
        let (result, output) =
            capture_host_output_with_stdin(None, || host_logger_debug(&[rv_str("visible")]));
        assert_eq!(result.unwrap(), rv_atom("ok"));
        assert_eq!(output.stderr, "[debug] visible\n");
        reset_level();
    }

    #[test]
    fn info_shown_at_info_level() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_level();
        let (result, output) =
            capture_host_output_with_stdin(None, || host_logger_info(&[rv_str("hello")]));
        assert_eq!(result.unwrap(), rv_atom("ok"));
        assert_eq!(output.stderr, "[info] hello\n");
    }

    #[test]
    fn warn_shown_at_info_level() {
        let _g = TEST_LOCK.lock().unwrap();
        reset_level();
        let (result, output) =
            capture_host_output_with_stdin(None, || host_logger_warn(&[rv_str("careful")]));
        assert_eq!(result.unwrap(), rv_atom("ok"));
        assert_eq!(output.stderr, "[warn] careful\n");
    }

    #[test]
    fn error_shown_at_error_level() {
        let _g = TEST_LOCK.lock().unwrap();
        LOG_LEVEL.store(LEVEL_ERROR, Ordering::Relaxed);
        let (result, output) =
            capture_host_output_with_stdin(None, || host_logger_error(&[rv_str("boom")]));
        assert_eq!(result.unwrap(), rv_atom("ok"));
        assert_eq!(output.stderr, "[error] boom\n");
        reset_level();
    }

    #[test]
    fn warn_suppressed_at_error_level() {
        let _g = TEST_LOCK.lock().unwrap();
        LOG_LEVEL.store(LEVEL_ERROR, Ordering::Relaxed);
        let (result, output) =
            capture_host_output_with_stdin(None, || host_logger_warn(&[rv_str("hidden")]));
        assert_eq!(result.unwrap(), rv_atom("ok"));
        assert_eq!(output.stderr, "");
        reset_level();
    }

    #[test]
    fn none_level_suppresses_everything() {
        let _g = TEST_LOCK.lock().unwrap();
        LOG_LEVEL.store(LEVEL_NONE, Ordering::Relaxed);
        let (_r1, o1) =
            capture_host_output_with_stdin(None, || host_logger_error(&[rv_str("hidden")]));
        assert_eq!(o1.stderr, "");
        reset_level();
    }

    #[test]
    fn debug_rejects_wrong_arity() {
        let result = host_logger_debug(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn debug_rejects_non_string() {
        let result = host_logger_debug(&[RuntimeValue::Int(42)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("string"));
    }

    #[test]
    fn get_level_rejects_wrong_arity() {
        let result = host_logger_get_level(&[rv_atom("info")]);
        assert!(result.is_err());
    }

    #[test]
    fn set_level_rejects_wrong_arity() {
        let result = host_logger_set_level(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn registration_makes_functions_callable() {
        let registry = HostRegistry::new();
        register_logger_host_functions(&registry);
        assert!(registry.call("logger_info", &[rv_str("test")]).is_ok());
    }
}
