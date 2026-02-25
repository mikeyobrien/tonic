//! Host interop module for Tonic
//!
//! Provides a static extension registry for calling Rust host functions from Tonic code.
//! v1 uses a static registry model (no dynamic plugin loading).

use crate::runtime::RuntimeValue;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

mod system;

/// Host function signature: takes runtime values, returns result
pub type HostFn = fn(&[RuntimeValue]) -> Result<RuntimeValue, HostError>;

/// Errors that can occur during host function execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "host error: {}", self.message)
    }
}

impl std::error::Error for HostError {}

pub(super) fn host_value_kind(value: &RuntimeValue) -> &'static str {
    match value {
        RuntimeValue::Int(_) => "int",
        RuntimeValue::Float(_) => "float",
        RuntimeValue::Bool(_) => "bool",
        RuntimeValue::Nil => "nil",
        RuntimeValue::String(_) => "string",
        RuntimeValue::Atom(_) => "atom",
        RuntimeValue::ResultOk(_) | RuntimeValue::ResultErr(_) => "result",
        RuntimeValue::Tuple(_, _) => "tuple",
        RuntimeValue::Map(_) => "map",
        RuntimeValue::Keyword(_) => "keyword",
        RuntimeValue::List(_) => "list",
        RuntimeValue::Range(_, _) => "range",
        RuntimeValue::Closure(_) => "function",
    }
}

/// Static registry for host functions
pub struct HostRegistry {
    functions: Mutex<HashMap<String, HostFn>>,
}

impl HostRegistry {
    pub fn new() -> Self {
        let registry = Self {
            functions: Mutex::new(HashMap::new()),
        };
        registry.register_sample_functions();
        registry
    }

    /// Register a host function with an atom key
    pub fn register(&self, key: impl Into<String>, function: HostFn) {
        let mut functions = self.functions.lock().unwrap();
        functions.insert(key.into(), function);
    }

    /// Look up and invoke a host function by atom key
    pub fn call(&self, key: &str, args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
        let functions = self.functions.lock().unwrap();
        let function = functions
            .get(key)
            .ok_or_else(|| HostError::new(format!("unknown host function: {key}")))?;
        function(args)
    }

    /// Register sample host functions for testing and tooling interop.
    fn register_sample_functions(&self) {
        // :identity - returns its single argument unchanged
        self.register("identity", |args| {
            if args.len() != 1 {
                return Err(HostError::new(format!(
                    "identity expects exactly 1 argument, found {}",
                    args.len()
                )));
            }

            Ok(args[0].clone())
        });

        // :sum_ints - sums integer arguments with strict validation
        self.register("sum_ints", |args| {
            if args.is_empty() {
                return Err(HostError::new("sum_ints expects at least 1 argument"));
            }

            let mut sum = 0i64;
            for (index, value) in args.iter().enumerate() {
                match value {
                    RuntimeValue::Int(number) => sum += number,
                    other => {
                        return Err(HostError::new(format!(
                            "sum_ints expects int arguments only; argument {} was {}",
                            index + 1,
                            host_value_kind(other)
                        )));
                    }
                }
            }

            Ok(RuntimeValue::Int(sum))
        });

        // :make_error - always returns an error
        self.register("make_error", |args| {
            let message = args
                .first()
                .map(|v| v.render())
                .unwrap_or_else(|| "unknown error".to_string());
            Err(HostError::new(message))
        });

        // System interop primitives for tonicctl and similar tooling.
        system::register_system_host_functions(self);
    }
}

impl Default for HostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global host registry instance
pub static HOST_REGISTRY: LazyLock<HostRegistry> = LazyLock::new(HostRegistry::new);

#[cfg(test)]
mod tests {
    use super::*;

    fn map_lookup<'a>(map: &'a RuntimeValue, key: &str) -> Option<&'a RuntimeValue> {
        let RuntimeValue::Map(entries) = map else {
            return None;
        };

        entries.iter().find_map(|(entry_key, entry_value)| {
            if matches!(entry_key, RuntimeValue::Atom(atom) if atom == key) {
                Some(entry_value)
            } else {
                None
            }
        })
    }

    #[test]
    fn host_registry_registers_and_calls_functions() {
        let registry = HostRegistry::new();

        // Register a simple function
        registry.register("double", |args| {
            if let Some(RuntimeValue::Int(n)) = args.first() {
                Ok(RuntimeValue::Int(n * 2))
            } else {
                Err(HostError::new("expected int argument"))
            }
        });

        // Call it
        let result = registry.call("double", &[RuntimeValue::Int(5)]);
        assert_eq!(result, Ok(RuntimeValue::Int(10)));
    }

    #[test]
    fn host_registry_reports_unknown_function() {
        let registry = HostRegistry::new();

        let result = registry.call("nonexistent", &[]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "host error: unknown host function: nonexistent"
        );
    }

    #[test]
    fn host_registry_sample_identity_works() {
        let result = HOST_REGISTRY.call("identity", &[RuntimeValue::Int(42)]);
        assert_eq!(result, Ok(RuntimeValue::Int(42)));
    }

    #[test]
    fn host_registry_sample_sum_ints_works() {
        let result = HOST_REGISTRY.call(
            "sum_ints",
            &[
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ],
        );
        assert_eq!(result, Ok(RuntimeValue::Int(6)));
    }

    #[test]
    fn host_registry_sample_identity_rejects_wrong_arity() {
        let zero_args = HOST_REGISTRY
            .call("identity", &[])
            .expect_err("identity should reject calls that do not provide exactly one argument");
        assert_eq!(
            zero_args.to_string(),
            "host error: identity expects exactly 1 argument, found 0"
        );

        let two_args = HOST_REGISTRY
            .call("identity", &[RuntimeValue::Int(1), RuntimeValue::Int(2)])
            .expect_err("identity should reject calls with more than one argument");
        assert_eq!(
            two_args.to_string(),
            "host error: identity expects exactly 1 argument, found 2"
        );
    }

    #[test]
    fn host_registry_sample_sum_ints_rejects_invalid_arguments() {
        let zero_args = HOST_REGISTRY
            .call("sum_ints", &[])
            .expect_err("sum_ints should reject empty argument lists");
        assert_eq!(
            zero_args.to_string(),
            "host error: sum_ints expects at least 1 argument"
        );

        let mixed = HOST_REGISTRY
            .call(
                "sum_ints",
                &[RuntimeValue::Int(1), RuntimeValue::Atom("oops".to_string())],
            )
            .expect_err("sum_ints should reject non-int arguments");
        assert_eq!(
            mixed.to_string(),
            "host error: sum_ints expects int arguments only; argument 2 was atom"
        );
    }

    #[test]
    fn host_registry_sample_make_error_works() {
        let result = HOST_REGISTRY.call(
            "make_error",
            &[RuntimeValue::String("test error".to_string())],
        );
        assert!(result.is_err());
    }

    #[test]
    fn host_registry_system_path_exists_and_write_text_work() {
        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-interop-system-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let target_dir = fixture_root.join("nested");
        let target_file = target_dir.join("report.txt");

        let ensure_result = HOST_REGISTRY
            .call(
                "sys_ensure_dir",
                &[RuntimeValue::String(target_dir.display().to_string())],
            )
            .expect("sys_ensure_dir should succeed");
        assert_eq!(ensure_result, RuntimeValue::Bool(true));

        let write_result = HOST_REGISTRY
            .call(
                "sys_write_text",
                &[
                    RuntimeValue::String(target_file.display().to_string()),
                    RuntimeValue::String("hello".to_string()),
                ],
            )
            .expect("sys_write_text should succeed");
        assert_eq!(write_result, RuntimeValue::Bool(true));

        let exists_result = HOST_REGISTRY
            .call(
                "sys_path_exists",
                &[RuntimeValue::String(target_file.display().to_string())],
            )
            .expect("sys_path_exists should succeed");
        assert_eq!(exists_result, RuntimeValue::Bool(true));

        let content =
            std::fs::read_to_string(&target_file).expect("report file should be readable");
        assert_eq!(content, "hello");

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_env_and_which_work() {
        let missing = HOST_REGISTRY
            .call(
                "sys_env",
                &[RuntimeValue::String(
                    "TONIC_INTEROP_MISSING_KEY".to_string(),
                )],
            )
            .expect("sys_env should succeed for missing values");
        assert_eq!(missing, RuntimeValue::Nil);

        let shell_path = HOST_REGISTRY
            .call("sys_which", &[RuntimeValue::String("sh".to_string())])
            .expect("sys_which should succeed");

        assert!(
            matches!(shell_path, RuntimeValue::String(_) | RuntimeValue::Nil),
            "expected string-or-nil from sys_which, got {:?}",
            shell_path
        );

        let cwd = HOST_REGISTRY
            .call("sys_cwd", &[])
            .expect("sys_cwd should succeed");
        assert!(matches!(cwd, RuntimeValue::String(_)));
    }

    #[test]
    fn host_registry_system_run_returns_exit_code_and_output() {
        let success = HOST_REGISTRY
            .call(
                "sys_run",
                &[RuntimeValue::String("printf 'hello'".to_string())],
            )
            .expect("sys_run should succeed for valid command");

        assert_eq!(
            map_lookup(&success, "exit_code"),
            Some(&RuntimeValue::Int(0))
        );
        assert_eq!(
            map_lookup(&success, "output"),
            Some(&RuntimeValue::String("hello".to_string()))
        );

        let failure = HOST_REGISTRY
            .call("sys_run", &[RuntimeValue::String("exit 3".to_string())])
            .expect("sys_run should still return map for non-zero exit");

        assert_eq!(
            map_lookup(&failure, "exit_code"),
            Some(&RuntimeValue::Int(3))
        );
    }
}
