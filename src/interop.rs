//! Host interop module for Tonic
//!
//! Provides a static extension registry for calling Rust host functions from Tonic code.
//! v1 uses a static registry model (no dynamic plugin loading).

use crate::runtime::RuntimeValue;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

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

fn host_value_kind(value: &RuntimeValue) -> &'static str {
    match value {
        RuntimeValue::Int(_) => "int",
        RuntimeValue::Float(_) => "float",
        RuntimeValue::Bool(_) => "bool",
        RuntimeValue::Nil => "nil",
        RuntimeValue::String(_) => "string",
        RuntimeValue::Atom(_) => "atom",
        RuntimeValue::ResultOk(_) | RuntimeValue::ResultErr(_) => "result",
        RuntimeValue::Tuple(_, _) => "tuple",
        RuntimeValue::Map(_, _) => "map",
        RuntimeValue::Keyword(_, _) => "keyword",
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

    /// Check if a host function exists
    pub fn contains(&self, key: &str) -> bool {
        let functions = self.functions.lock().unwrap();
        functions.contains_key(key)
    }

    /// Register sample host functions for testing
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
}
