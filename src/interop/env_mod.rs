use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

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

fn expect_string_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<String, HostError> {
    match &args[index] {
        RuntimeValue::String(s) => Ok(s.clone()),
        other => Err(HostError::new(format!(
            "{} expects a string argument at position {}, got {:?}",
            function,
            index + 1,
            std::mem::discriminant(other)
        ))),
    }
}

/// Set an environment variable.
fn env_set(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Env.set", args, 2)?;
    let key = expect_string_arg("Env.set", args, 0)?;
    let value = expect_string_arg("Env.set", args, 1)?;
    // SAFETY: We are single-threaded in the Tonic runtime context.
    unsafe { std::env::set_var(&key, &value) };
    Ok(RuntimeValue::Atom("ok".to_string()))
}

/// Delete (unset) an environment variable.
fn env_delete(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Env.delete", args, 1)?;
    let key = expect_string_arg("Env.delete", args, 0)?;
    // SAFETY: We are single-threaded in the Tonic runtime context.
    unsafe { std::env::remove_var(&key) };
    Ok(RuntimeValue::Atom("ok".to_string()))
}

/// Return all environment variables as a map of string→string.
fn env_all(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Env.all", args, 0)?;
    let entries: Vec<(RuntimeValue, RuntimeValue)> = std::env::vars()
        .map(|(k, v)| (RuntimeValue::String(k), RuntimeValue::String(v)))
        .collect();
    Ok(RuntimeValue::Map(entries))
}

/// Check whether an environment variable is set.
fn env_has_key(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Env.has_key", args, 1)?;
    let key = expect_string_arg("Env.has_key", args, 0)?;
    let exists = std::env::var_os(&key).is_some();
    Ok(RuntimeValue::Bool(exists))
}

pub(crate) fn register_env_host_functions(registry: &HostRegistry) {
    registry.register("env_set", env_set);
    registry.register("env_delete", env_delete);
    registry.register("env_all", env_all);
    registry.register("env_has_key", env_has_key);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_round_trip() {
        let key = "_TONIC_ENV_TEST_SET_GET";
        // Set
        let result = env_set(&[
            RuntimeValue::String(key.to_string()),
            RuntimeValue::String("hello".to_string()),
        ]);
        assert_eq!(result.unwrap(), RuntimeValue::Atom("ok".to_string()));
        // Verify via std::env
        assert_eq!(std::env::var(key).unwrap(), "hello");
        // Cleanup
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn delete_removes_variable() {
        let key = "_TONIC_ENV_TEST_DELETE";
        unsafe { std::env::set_var(key, "to_delete") };
        let result = env_delete(&[RuntimeValue::String(key.to_string())]);
        assert_eq!(result.unwrap(), RuntimeValue::Atom("ok".to_string()));
        assert!(std::env::var_os(key).is_none());
    }

    #[test]
    fn delete_nonexistent_is_ok() {
        let result = env_delete(&[RuntimeValue::String(
            "_TONIC_ENV_NONEXISTENT_XYZ".to_string(),
        )]);
        assert_eq!(result.unwrap(), RuntimeValue::Atom("ok".to_string()));
    }

    #[test]
    fn all_returns_map_with_known_key() {
        let key = "_TONIC_ENV_TEST_ALL";
        unsafe { std::env::set_var(key, "present") };
        let result = env_all(&[]).unwrap();
        match &result {
            RuntimeValue::Map(entries) => {
                let target = RuntimeValue::String(key.to_string());
                let val = entries.iter().find(|(k, _)| k == &target).map(|(_, v)| v);
                assert_eq!(val, Some(&RuntimeValue::String("present".to_string())));
            }
            other => panic!("expected map, got {other:?}"),
        }
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn all_returns_nonempty_map() {
        let result = env_all(&[]).unwrap();
        match &result {
            RuntimeValue::Map(m) => assert!(!m.is_empty(), "env map should not be empty"),
            other => panic!("expected map, got {other:?}"),
        }
    }

    #[test]
    fn has_key_returns_true_for_existing() {
        let key = "_TONIC_ENV_TEST_HASKEY";
        unsafe { std::env::set_var(key, "yes") };
        let result = env_has_key(&[RuntimeValue::String(key.to_string())]).unwrap();
        assert_eq!(result, RuntimeValue::Bool(true));
        unsafe { std::env::remove_var(key) };
    }

    #[test]
    fn has_key_returns_false_for_missing() {
        let result = env_has_key(&[RuntimeValue::String(
            "_TONIC_ENV_DEFINITELY_NOT_SET".to_string(),
        )])
        .unwrap();
        assert_eq!(result, RuntimeValue::Bool(false));
    }

    #[test]
    fn set_rejects_non_string_key() {
        let err =
            env_set(&[RuntimeValue::Int(42), RuntimeValue::String("v".to_string())]).unwrap_err();
        assert!(err.message.contains("string argument"));
    }

    #[test]
    fn set_rejects_non_string_value() {
        let err =
            env_set(&[RuntimeValue::String("k".to_string()), RuntimeValue::Int(42)]).unwrap_err();
        assert!(err.message.contains("string argument"));
    }

    #[test]
    fn set_rejects_wrong_arity() {
        let err = env_set(&[RuntimeValue::String("k".to_string())]).unwrap_err();
        assert!(err.message.contains("expects exactly 2"));
    }

    #[test]
    fn has_key_rejects_wrong_arity() {
        let err = env_has_key(&[]).unwrap_err();
        assert!(err.message.contains("expects exactly 1"));
    }

    #[test]
    fn all_rejects_arguments() {
        let err = env_all(&[RuntimeValue::Int(1)]).unwrap_err();
        assert!(err.message.contains("expects exactly 0"));
    }

    #[test]
    fn register_adds_all_functions() {
        let registry = HostRegistry::new();
        assert!(registry
            .call(
                "env_set",
                &[
                    RuntimeValue::String("k".to_string()),
                    RuntimeValue::String("v".to_string())
                ]
            )
            .is_ok());
        assert!(registry
            .call("env_delete", &[RuntimeValue::String("k".to_string())])
            .is_ok());
        assert!(registry.call("env_all", &[]).is_ok());
        assert!(registry
            .call("env_has_key", &[RuntimeValue::String("k".to_string())])
            .is_ok());
    }
}
