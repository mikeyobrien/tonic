//! Host interop module for Tonic
//!
//! Provides a static extension registry for calling Rust host functions from Tonic code.
//! v1 uses a static registry model (no dynamic plugin loading).

use crate::runtime::RuntimeValue;
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

mod http_server;
mod path_mod;
mod string_mod;
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
        RuntimeValue::SteppedRange(_, _, _) => "stepped_range",
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

        // String stdlib interop primitives for interpreter-backed String.* calls.
        string_mod::register_string_host_functions(self);

        // Path stdlib interop primitives for interpreter-backed Path.* calls.
        path_mod::register_path_host_functions(self);

        // HTTP server primitives for tonic-only server code.
        http_server::register_http_server_host_functions(self);
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
    use std::sync::{LazyLock, Mutex};

    static SYSTEM_LOG_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

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

    #[test]
    fn host_registry_system_read_text_reports_missing_file() {
        let missing = HOST_REGISTRY
            .call(
                "sys_read_text",
                &[RuntimeValue::String(
                    "/tmp/tonic-missing-file.txt".to_string(),
                )],
            )
            .expect_err("sys_read_text should report missing file");

        assert!(
            missing
                .to_string()
                .starts_with("host error: sys_read_text failed for '/tmp/tonic-missing-file.txt':"),
            "expected deterministic read_text error prefix, got: {}",
            missing
        );
    }

    #[test]
    fn host_registry_system_http_request_rejects_invalid_method() {
        let invalid_method = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("TRACE".to_string()),
                    RuntimeValue::String("https://example.com".to_string()),
                    RuntimeValue::List(Vec::new()),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(Vec::new()),
                ],
            )
            .expect_err("sys_http_request should reject unsupported methods");

        assert_eq!(
            invalid_method.to_string(),
            "host error: sys_http_request invalid method: TRACE"
        );
    }

    #[test]
    fn host_registry_system_http_request_rejects_unknown_opts_key() {
        let invalid_opts = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("https://example.com".to_string()),
                    RuntimeValue::List(Vec::new()),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(vec![(
                        RuntimeValue::Atom("surprise".to_string()),
                        RuntimeValue::Bool(true),
                    )]),
                ],
            )
            .expect_err("sys_http_request should reject unknown opts keys");

        assert_eq!(
            invalid_opts.to_string(),
            "host error: sys_http_request unsupported opts key: surprise"
        );
    }

    #[test]
    fn host_registry_system_read_text_rejects_non_string_path() {
        let wrong_type = HOST_REGISTRY
            .call("sys_read_text", &[RuntimeValue::Int(42)])
            .expect_err("sys_read_text should reject non-string argument");

        assert_eq!(
            wrong_type.to_string(),
            "host error: sys_read_text expects string argument 1; found int"
        );
    }

    #[test]
    fn host_registry_system_read_text_reads_written_file() {
        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-read-text-roundtrip-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&fixture_root).expect("fixture dir should be created");

        let file_path = fixture_root.join("sample.txt");
        std::fs::write(&file_path, "hello world").expect("fixture file should be writable");

        let result = HOST_REGISTRY
            .call(
                "sys_read_text",
                &[RuntimeValue::String(file_path.display().to_string())],
            )
            .expect("sys_read_text should succeed for existing file");

        assert_eq!(result, RuntimeValue::String("hello world".to_string()));

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_list_files_recursive_returns_sorted_relative_paths() {
        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-list-files-recursive-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let nested_dir = fixture_root.join("docs").join("guide");
        std::fs::create_dir_all(&nested_dir).expect("fixture dirs should be created");
        std::fs::write(fixture_root.join("z-style.css"), "root")
            .expect("root fixture file should be writable");
        std::fs::write(nested_dir.join("intro.css"), "nested")
            .expect("nested fixture file should be writable");

        let result = HOST_REGISTRY
            .call(
                "sys_list_files_recursive",
                &[RuntimeValue::String(fixture_root.display().to_string())],
            )
            .expect("sys_list_files_recursive should succeed for existing directory");

        assert_eq!(
            result,
            RuntimeValue::List(vec![
                RuntimeValue::String("docs/guide/intro.css".to_string()),
                RuntimeValue::String("z-style.css".to_string()),
            ])
        );

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_remove_tree_removes_nested_tree_and_reports_missing() {
        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-remove-tree-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let nested_dir = fixture_root.join("out with space").join("docs");
        std::fs::create_dir_all(&nested_dir).expect("fixture dirs should be created");
        std::fs::write(nested_dir.join("guide.css"), "nested")
            .expect("nested fixture file should be writable");

        let target_path = fixture_root.join("out with space");
        let target_display = target_path.display().to_string();

        let removed = HOST_REGISTRY
            .call(
                "sys_remove_tree",
                &[RuntimeValue::String(target_display.clone())],
            )
            .expect("sys_remove_tree should succeed for existing directory");
        assert_eq!(removed, RuntimeValue::Bool(true));
        assert!(
            !target_path.exists(),
            "expected remove_tree target to be removed at {}",
            target_path.display()
        );

        let missing = HOST_REGISTRY
            .call("sys_remove_tree", &[RuntimeValue::String(target_display)])
            .expect("sys_remove_tree should return false for missing path");
        assert_eq!(missing, RuntimeValue::Bool(false));

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_append_text_and_write_text_atomic_persist_expected_content() {
        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-system-persistence-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));

        let target_file = fixture_root.join("state").join("proposal-log.txt");
        let target_path = target_file.display().to_string();

        let first_append = HOST_REGISTRY
            .call(
                "sys_append_text",
                &[
                    RuntimeValue::String(target_path.clone()),
                    RuntimeValue::String("proposal-1\n".to_string()),
                ],
            )
            .expect("first append should succeed");
        let second_append = HOST_REGISTRY
            .call(
                "sys_append_text",
                &[
                    RuntimeValue::String(target_path.clone()),
                    RuntimeValue::String("proposal-2\n".to_string()),
                ],
            )
            .expect("second append should succeed");

        assert_eq!(first_append, RuntimeValue::Bool(true));
        assert_eq!(second_append, RuntimeValue::Bool(true));

        let appended = std::fs::read_to_string(&target_file)
            .expect("append target should be readable after append writes");
        assert_eq!(appended, "proposal-1\nproposal-2\n");

        let atomic_write = HOST_REGISTRY
            .call(
                "sys_write_text_atomic",
                &[
                    RuntimeValue::String(target_path),
                    RuntimeValue::String("snapshot-v2".to_string()),
                ],
            )
            .expect("atomic write should succeed");

        assert_eq!(atomic_write, RuntimeValue::Bool(true));

        let replaced = std::fs::read_to_string(&target_file)
            .expect("atomic write target should be readable after replace");
        assert_eq!(replaced, "snapshot-v2");

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_lock_acquire_and_release_are_deterministic() {
        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-system-lock-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let lock_path = fixture_root.join("locks").join("proposal.lock");
        let lock_path_value = RuntimeValue::String(lock_path.display().to_string());

        let acquired = HOST_REGISTRY
            .call("sys_lock_acquire", std::slice::from_ref(&lock_path_value))
            .expect("first lock acquisition should succeed");
        let blocked = HOST_REGISTRY
            .call("sys_lock_acquire", std::slice::from_ref(&lock_path_value))
            .expect("second lock acquisition should return false without error");

        assert_eq!(acquired, RuntimeValue::Bool(true));
        assert_eq!(blocked, RuntimeValue::Bool(false));

        let released = HOST_REGISTRY
            .call("sys_lock_release", std::slice::from_ref(&lock_path_value))
            .expect("first lock release should succeed");
        let missing_release = HOST_REGISTRY
            .call("sys_lock_release", &[lock_path_value])
            .expect("second lock release should return false without error");

        assert_eq!(released, RuntimeValue::Bool(true));
        assert_eq!(missing_release, RuntimeValue::Bool(false));

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_persistence_primitives_report_deterministic_argument_and_io_errors() {
        let append_wrong_type = HOST_REGISTRY
            .call(
                "sys_append_text",
                &[
                    RuntimeValue::Int(7),
                    RuntimeValue::String("payload".to_string()),
                ],
            )
            .expect_err("sys_append_text should reject non-string paths");

        assert_eq!(
            append_wrong_type.to_string(),
            "host error: sys_append_text expects string argument 1; found int"
        );

        let atomic_wrong_type = HOST_REGISTRY
            .call(
                "sys_write_text_atomic",
                &[
                    RuntimeValue::String("/tmp/demo.txt".to_string()),
                    RuntimeValue::Bool(true),
                ],
            )
            .expect_err("sys_write_text_atomic should reject non-string content");

        assert_eq!(
            atomic_wrong_type.to_string(),
            "host error: sys_write_text_atomic expects string argument 2; found bool"
        );

        let lock_wrong_type = HOST_REGISTRY
            .call("sys_lock_acquire", &[RuntimeValue::Int(1)])
            .expect_err("sys_lock_acquire should reject non-string lock paths");

        assert_eq!(
            lock_wrong_type.to_string(),
            "host error: sys_lock_acquire expects string argument 1; found int"
        );

        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-system-persistence-errors-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        std::fs::create_dir_all(&fixture_root).expect("fixture root should be created");

        let directory_target = fixture_root.join("directory-target");
        std::fs::create_dir_all(&directory_target)
            .expect("directory target should exist for io error assertions");
        let directory_target_text = directory_target.display().to_string();

        let append_io_error = HOST_REGISTRY
            .call(
                "sys_append_text",
                &[
                    RuntimeValue::String(directory_target_text.clone()),
                    RuntimeValue::String("payload".to_string()),
                ],
            )
            .expect_err("sys_append_text should report io errors for directory targets");
        let append_prefix = format!(
            "host error: sys_append_text failed for '{}':",
            directory_target.display()
        );
        assert!(
            append_io_error.to_string().starts_with(&append_prefix),
            "expected deterministic append io error prefix, got: {}",
            append_io_error
        );

        let atomic_io_error = HOST_REGISTRY
            .call(
                "sys_write_text_atomic",
                &[
                    RuntimeValue::String(directory_target_text),
                    RuntimeValue::String("payload".to_string()),
                ],
            )
            .expect_err("sys_write_text_atomic should report io errors for directory targets");
        let atomic_prefix = format!(
            "host error: sys_write_text_atomic failed for '{}':",
            directory_target.display()
        );
        assert!(
            atomic_io_error.to_string().starts_with(&atomic_prefix),
            "expected deterministic atomic io error prefix, got: {}",
            atomic_io_error
        );

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_http_request_rejects_invalid_url() {
        let invalid_url = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("not a url".to_string()),
                    RuntimeValue::List(Vec::new()),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(Vec::new()),
                ],
            )
            .expect_err("sys_http_request should reject invalid URL");

        assert_eq!(
            invalid_url.to_string(),
            "host error: sys_http_request invalid url: not a url"
        );
    }

    #[test]
    fn host_registry_system_http_request_rejects_unsupported_url_scheme() {
        let ftp_scheme = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("ftp://example.com/file".to_string()),
                    RuntimeValue::List(Vec::new()),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(Vec::new()),
                ],
            )
            .expect_err("sys_http_request should reject ftp scheme");

        assert_eq!(
            ftp_scheme.to_string(),
            "host error: sys_http_request unsupported url scheme: ftp"
        );
    }

    #[test]
    fn host_registry_system_http_request_rejects_timeout_out_of_range() {
        let too_low = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("https://example.com".to_string()),
                    RuntimeValue::List(Vec::new()),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(vec![(
                        RuntimeValue::Atom("timeout_ms".to_string()),
                        RuntimeValue::Int(50),
                    )]),
                ],
            )
            .expect_err("sys_http_request should reject timeout below minimum");

        assert_eq!(
            too_low.to_string(),
            "host error: sys_http_request timeout_ms out of range: 50"
        );

        let too_high = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("https://example.com".to_string()),
                    RuntimeValue::List(Vec::new()),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(vec![(
                        RuntimeValue::Atom("timeout_ms".to_string()),
                        RuntimeValue::Int(200_000),
                    )]),
                ],
            )
            .expect_err("sys_http_request should reject timeout above maximum");

        assert_eq!(
            too_high.to_string(),
            "host error: sys_http_request timeout_ms out of range: 200000"
        );
    }

    #[test]
    fn host_registry_system_http_request_rejects_invalid_header_entry() {
        let bad_header = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("https://example.com".to_string()),
                    RuntimeValue::List(vec![RuntimeValue::String("not-a-tuple".to_string())]),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(Vec::new()),
                ],
            )
            .expect_err("sys_http_request should reject non-tuple header entries");

        assert_eq!(
            bad_header.to_string(),
            "host error: sys_http_request headers argument 3 entry 1 must be {string, string}; found string"
        );
    }

    #[test]
    fn host_registry_system_http_request_rejects_wrong_arity() {
        let too_few = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("https://example.com".to_string()),
                ],
            )
            .expect_err("sys_http_request should reject wrong arity");

        assert_eq!(
            too_few.to_string(),
            "host error: sys_http_request expects exactly 5 arguments, found 2"
        );
    }

    #[test]
    fn host_registry_system_http_request_rejects_max_redirects_out_of_range() {
        let too_high = HOST_REGISTRY
            .call(
                "sys_http_request",
                &[
                    RuntimeValue::String("GET".to_string()),
                    RuntimeValue::String("https://example.com".to_string()),
                    RuntimeValue::List(Vec::new()),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::Map(vec![(
                        RuntimeValue::Atom("max_redirects".to_string()),
                        RuntimeValue::Int(10),
                    )]),
                ],
            )
            .expect_err("sys_http_request should reject max_redirects above cap");

        assert_eq!(
            too_high.to_string(),
            "host error: sys_http_request max_redirects out of range: 10"
        );
    }

    // ---- sys_sleep_ms + sys_retry_plan tests ----

    #[test]
    fn host_registry_system_sleep_ms_accepts_zero_delay() {
        let result = HOST_REGISTRY
            .call("sys_sleep_ms", &[RuntimeValue::Int(0)])
            .expect("sys_sleep_ms should accept zero delay");

        assert_eq!(result, RuntimeValue::Bool(true));
    }

    #[test]
    fn host_registry_system_retry_plan_uses_retry_after_and_caps_delay() {
        let result = HOST_REGISTRY
            .call(
                "sys_retry_plan",
                &[
                    RuntimeValue::Int(429),
                    RuntimeValue::Int(1),
                    RuntimeValue::Int(4),
                    RuntimeValue::Int(250),
                    RuntimeValue::Int(5_000),
                    RuntimeValue::Int(0),
                    RuntimeValue::String("120".to_string()),
                ],
            )
            .expect("sys_retry_plan should parse Retry-After seconds");

        assert_eq!(
            map_lookup(&result, "retry"),
            Some(&RuntimeValue::Bool(true))
        );
        assert_eq!(
            map_lookup(&result, "delay_ms"),
            Some(&RuntimeValue::Int(5_000))
        );
        assert_eq!(
            map_lookup(&result, "source"),
            Some(&RuntimeValue::Atom("retry_after".to_string()))
        );
    }

    #[test]
    fn host_registry_system_retry_plan_falls_back_to_backoff_for_invalid_retry_after() {
        let result = HOST_REGISTRY
            .call(
                "sys_retry_plan",
                &[
                    RuntimeValue::Int(429),
                    RuntimeValue::Int(2),
                    RuntimeValue::Int(5),
                    RuntimeValue::Int(250),
                    RuntimeValue::Int(5_000),
                    RuntimeValue::Int(0),
                    RuntimeValue::String("not-a-header".to_string()),
                ],
            )
            .expect("sys_retry_plan should fall back to backoff when header is invalid");

        assert_eq!(
            map_lookup(&result, "retry"),
            Some(&RuntimeValue::Bool(true))
        );
        assert_eq!(
            map_lookup(&result, "delay_ms"),
            Some(&RuntimeValue::Int(500))
        );
        assert_eq!(
            map_lookup(&result, "source"),
            Some(&RuntimeValue::Atom("backoff".to_string()))
        );
    }

    #[test]
    fn host_registry_system_retry_plan_stops_after_attempt_budget_is_exhausted() {
        let result = HOST_REGISTRY
            .call(
                "sys_retry_plan",
                &[
                    RuntimeValue::Int(429),
                    RuntimeValue::Int(4),
                    RuntimeValue::Int(4),
                    RuntimeValue::Int(250),
                    RuntimeValue::Int(5_000),
                    RuntimeValue::Int(0),
                    RuntimeValue::Nil,
                ],
            )
            .expect("sys_retry_plan should stop retries when attempt budget is exhausted");

        assert_eq!(
            map_lookup(&result, "retry"),
            Some(&RuntimeValue::Bool(false))
        );
        assert_eq!(map_lookup(&result, "delay_ms"), Some(&RuntimeValue::Int(0)));
        assert_eq!(
            map_lookup(&result, "source"),
            Some(&RuntimeValue::Atom("exhausted".to_string()))
        );
    }

    #[test]
    fn host_registry_system_retry_plan_is_deterministic_with_jitter() {
        let args = [
            RuntimeValue::Int(429),
            RuntimeValue::Int(2),
            RuntimeValue::Int(5),
            RuntimeValue::Int(250),
            RuntimeValue::Int(5_000),
            RuntimeValue::Int(100),
            RuntimeValue::Nil,
        ];

        let first = HOST_REGISTRY
            .call("sys_retry_plan", &args)
            .expect("first retry-plan call should succeed");
        let second = HOST_REGISTRY
            .call("sys_retry_plan", &args)
            .expect("second retry-plan call should succeed");

        assert_eq!(first, second, "retry planning must be deterministic");

        let Some(RuntimeValue::Int(delay_ms)) = map_lookup(&first, "delay_ms") else {
            panic!("expected delay_ms int in retry-plan result: {first:?}");
        };
        assert!(
            (500..=600).contains(delay_ms),
            "delay should include bounded deterministic jitter for attempt 2, got {delay_ms}"
        );
    }

    // ---- sys_log tests ----

    struct EnvVarGuard {
        key: &'static str,
        previous: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: String) -> Self {
            let previous = std::env::var(key).ok();
            std::env::set_var(key, value);
            Self { key, previous }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            if let Some(previous) = &self.previous {
                std::env::set_var(self.key, previous);
            } else {
                std::env::remove_var(self.key);
            }
        }
    }

    #[test]
    fn host_registry_system_log_appends_jsonl_records_with_expected_shape() {
        let _lock = SYSTEM_LOG_ENV_LOCK
            .lock()
            .expect("system log env lock should not be poisoned");

        let fixture_root = std::env::temp_dir().join(format!(
            "tonic-interop-system-log-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be after unix epoch")
                .as_nanos()
        ));
        let sink_path = fixture_root.join("audit").join("events.jsonl");
        let _env_guard = EnvVarGuard::set("TONIC_SYSTEM_LOG_PATH", sink_path.display().to_string());

        let first = HOST_REGISTRY
            .call(
                "sys_log",
                &[
                    RuntimeValue::Atom("info".to_string()),
                    RuntimeValue::String("triage.proposal_pending".to_string()),
                    RuntimeValue::Map(vec![
                        (
                            RuntimeValue::Atom("proposal_id".to_string()),
                            RuntimeValue::String("prop-123".to_string()),
                        ),
                        (
                            RuntimeValue::Atom("attempt".to_string()),
                            RuntimeValue::Int(1),
                        ),
                        (
                            RuntimeValue::Atom("meta".to_string()),
                            RuntimeValue::Map(vec![(
                                RuntimeValue::Atom("source".to_string()),
                                RuntimeValue::String("discord".to_string()),
                            )]),
                        ),
                        (
                            RuntimeValue::Atom("tags".to_string()),
                            RuntimeValue::List(vec![
                                RuntimeValue::String("triage".to_string()),
                                RuntimeValue::String("pending".to_string()),
                            ]),
                        ),
                    ]),
                ],
            )
            .expect("first sys_log call should succeed");

        let second = HOST_REGISTRY
            .call(
                "sys_log",
                &[
                    RuntimeValue::String("warn".to_string()),
                    RuntimeValue::String("triage.proposal_approved".to_string()),
                    RuntimeValue::Map(vec![
                        (
                            RuntimeValue::Atom("proposal_id".to_string()),
                            RuntimeValue::String("prop-123".to_string()),
                        ),
                        (
                            RuntimeValue::Atom("maintainer_id".to_string()),
                            RuntimeValue::String("u-42".to_string()),
                        ),
                    ]),
                ],
            )
            .expect("second sys_log call should append to same sink");

        assert_eq!(first, RuntimeValue::Bool(true));
        assert_eq!(second, RuntimeValue::Bool(true));

        let sink = std::fs::read_to_string(&sink_path)
            .expect("structured log sink should be readable after writes");
        let lines = sink.lines().collect::<Vec<_>>();
        assert_eq!(
            lines.len(),
            2,
            "sys_log should append one JSONL line per call"
        );

        let first_json: serde_json::Value =
            serde_json::from_str(lines[0]).expect("first log line should be valid JSON");
        let second_json: serde_json::Value =
            serde_json::from_str(lines[1]).expect("second log line should be valid JSON");

        assert_eq!(first_json["level"], serde_json::json!("info"));
        assert_eq!(
            first_json["event"],
            serde_json::json!("triage.proposal_pending")
        );
        assert_eq!(
            first_json["fields"]["proposal_id"],
            serde_json::json!("prop-123")
        );
        assert_eq!(first_json["fields"]["attempt"], serde_json::json!(1));
        assert_eq!(
            first_json["fields"]["meta"]["source"],
            serde_json::json!("discord")
        );
        assert_eq!(
            first_json["fields"]["tags"],
            serde_json::json!(["triage", "pending"])
        );
        assert!(
            first_json["timestamp_ms"].as_i64().is_some(),
            "log payload should include numeric timestamp_ms"
        );

        assert_eq!(second_json["level"], serde_json::json!("warn"));
        assert_eq!(
            second_json["event"],
            serde_json::json!("triage.proposal_approved")
        );
        assert_eq!(
            second_json["fields"]["maintainer_id"],
            serde_json::json!("u-42")
        );

        let _ = std::fs::remove_dir_all(&fixture_root);
    }

    #[test]
    fn host_registry_system_log_rejects_invalid_payloads_with_deterministic_errors() {
        let invalid_level = HOST_REGISTRY
            .call(
                "sys_log",
                &[
                    RuntimeValue::String("trace".to_string()),
                    RuntimeValue::String("triage.event".to_string()),
                    RuntimeValue::Map(Vec::new()),
                ],
            )
            .expect_err("sys_log should reject unsupported level values");

        assert_eq!(
            invalid_level.to_string(),
            "host error: sys_log level must be one of debug|info|warn|error; found trace"
        );

        let invalid_field_key = HOST_REGISTRY
            .call(
                "sys_log",
                &[
                    RuntimeValue::String("info".to_string()),
                    RuntimeValue::String("triage.event".to_string()),
                    RuntimeValue::Map(vec![(RuntimeValue::Int(1), RuntimeValue::Int(42))]),
                ],
            )
            .expect_err("sys_log should reject non atom/string field keys");

        assert_eq!(
            invalid_field_key.to_string(),
            "host error: sys_log fields key at entry 1 must be atom or string; found int"
        );
    }

    // ---- sys_random_token tests ----

    #[test]
    fn host_registry_system_random_token_returns_url_safe_base64() {
        let result = HOST_REGISTRY
            .call("sys_random_token", &[RuntimeValue::Int(32)])
            .expect("sys_random_token should succeed for valid byte count");

        let RuntimeValue::String(token) = result else {
            panic!("expected string result from sys_random_token");
        };

        assert!(
            token
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "token should contain only base64url characters, got: {token}"
        );
    }

    #[test]
    fn host_registry_system_random_token_has_correct_output_length() {
        // 32 bytes → ceil(32 * 4 / 3) = 43 chars in base64url unpadded
        let result = HOST_REGISTRY
            .call("sys_random_token", &[RuntimeValue::Int(32)])
            .expect("sys_random_token should succeed");

        let RuntimeValue::String(token) = result else {
            panic!("expected string result");
        };

        assert_eq!(
            token.len(),
            43,
            "32 bytes should produce 43 base64url chars, got {}",
            token.len()
        );

        // 16 bytes → 22 chars
        let result16 = HOST_REGISTRY
            .call("sys_random_token", &[RuntimeValue::Int(16)])
            .expect("sys_random_token should succeed for 16 bytes");

        let RuntimeValue::String(token16) = result16 else {
            panic!("expected string result");
        };

        assert_eq!(
            token16.len(),
            22,
            "16 bytes should produce 22 base64url chars, got {}",
            token16.len()
        );
    }

    #[test]
    fn host_registry_system_random_token_produces_unique_outputs() {
        let result1 = HOST_REGISTRY
            .call("sys_random_token", &[RuntimeValue::Int(32)])
            .expect("first call should succeed");
        let result2 = HOST_REGISTRY
            .call("sys_random_token", &[RuntimeValue::Int(32)])
            .expect("second call should succeed");

        assert_ne!(result1, result2, "two random tokens should not be equal");
    }

    #[test]
    fn host_registry_system_random_token_rejects_bytes_below_minimum() {
        let error = HOST_REGISTRY
            .call("sys_random_token", &[RuntimeValue::Int(8)])
            .expect_err("sys_random_token should reject byte count below minimum");

        assert_eq!(
            error.to_string(),
            "host error: sys_random_token bytes out of range: 8"
        );
    }

    #[test]
    fn host_registry_system_random_token_rejects_bytes_above_maximum() {
        let error = HOST_REGISTRY
            .call("sys_random_token", &[RuntimeValue::Int(512)])
            .expect_err("sys_random_token should reject byte count above maximum");

        assert_eq!(
            error.to_string(),
            "host error: sys_random_token bytes out of range: 512"
        );
    }

    #[test]
    fn host_registry_system_random_token_rejects_non_int_argument() {
        let error = HOST_REGISTRY
            .call(
                "sys_random_token",
                &[RuntimeValue::String("32".to_string())],
            )
            .expect_err("sys_random_token should reject non-int argument");

        assert_eq!(
            error.to_string(),
            "host error: sys_random_token expects int argument 1; found string"
        );
    }

    #[test]
    fn host_registry_system_random_token_rejects_wrong_arity() {
        let error = HOST_REGISTRY
            .call("sys_random_token", &[])
            .expect_err("sys_random_token should reject zero arguments");

        assert_eq!(
            error.to_string(),
            "host error: sys_random_token expects exactly 1 argument, found 0"
        );
    }

    // ---- sys_hmac_sha256_hex tests ----

    #[test]
    fn host_registry_system_hmac_sha256_hex_matches_known_test_vector() {
        // RFC 4231 Test Case 2
        let result = HOST_REGISTRY
            .call(
                "sys_hmac_sha256_hex",
                &[
                    RuntimeValue::String("Jefe".to_string()),
                    RuntimeValue::String("what do ya want for nothing?".to_string()),
                ],
            )
            .expect("sys_hmac_sha256_hex should succeed for valid inputs");

        assert_eq!(
            result,
            RuntimeValue::String(
                "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843".to_string()
            )
        );
    }

    #[test]
    fn host_registry_system_hmac_sha256_hex_produces_64_char_lowercase_hex() {
        let result = HOST_REGISTRY
            .call(
                "sys_hmac_sha256_hex",
                &[
                    RuntimeValue::String("secret".to_string()),
                    RuntimeValue::String("message".to_string()),
                ],
            )
            .expect("sys_hmac_sha256_hex should succeed");

        let RuntimeValue::String(hex) = result else {
            panic!("expected string result from sys_hmac_sha256_hex");
        };

        assert_eq!(
            hex.len(),
            64,
            "HMAC-SHA256 hex digest should be exactly 64 chars"
        );
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
            "digest should be lowercase hex only, got: {hex}"
        );
    }

    #[test]
    fn host_registry_system_hmac_sha256_hex_rejects_empty_secret() {
        let error = HOST_REGISTRY
            .call(
                "sys_hmac_sha256_hex",
                &[
                    RuntimeValue::String(String::new()),
                    RuntimeValue::String("message".to_string()),
                ],
            )
            .expect_err("sys_hmac_sha256_hex should reject empty secret");

        assert_eq!(
            error.to_string(),
            "host error: sys_hmac_sha256_hex secret must not be empty"
        );
    }

    #[test]
    fn host_registry_system_hmac_sha256_hex_rejects_empty_message() {
        let error = HOST_REGISTRY
            .call(
                "sys_hmac_sha256_hex",
                &[
                    RuntimeValue::String("secret".to_string()),
                    RuntimeValue::String(String::new()),
                ],
            )
            .expect_err("sys_hmac_sha256_hex should reject empty message");

        assert_eq!(
            error.to_string(),
            "host error: sys_hmac_sha256_hex message must not be empty"
        );
    }

    #[test]
    fn host_registry_system_hmac_sha256_hex_rejects_non_string_secret() {
        let error = HOST_REGISTRY
            .call(
                "sys_hmac_sha256_hex",
                &[
                    RuntimeValue::Int(42),
                    RuntimeValue::String("message".to_string()),
                ],
            )
            .expect_err("sys_hmac_sha256_hex should reject non-string secret");

        assert_eq!(
            error.to_string(),
            "host error: sys_hmac_sha256_hex expects string argument 1; found int"
        );
    }

    #[test]
    fn host_registry_system_hmac_sha256_hex_rejects_non_string_message() {
        let error = HOST_REGISTRY
            .call(
                "sys_hmac_sha256_hex",
                &[
                    RuntimeValue::String("secret".to_string()),
                    RuntimeValue::Bool(true),
                ],
            )
            .expect_err("sys_hmac_sha256_hex should reject non-string message");

        assert_eq!(
            error.to_string(),
            "host error: sys_hmac_sha256_hex expects string argument 2; found bool"
        );
    }

    #[test]
    fn host_registry_system_hmac_sha256_hex_rejects_wrong_arity() {
        let error = HOST_REGISTRY
            .call(
                "sys_hmac_sha256_hex",
                &[RuntimeValue::String("only-one".to_string())],
            )
            .expect_err("sys_hmac_sha256_hex should reject wrong arity");

        assert_eq!(
            error.to_string(),
            "host error: sys_hmac_sha256_hex expects exactly 2 arguments, found 1"
        );
    }

    // ---- sys_constant_time_eq + sys_discord_ed25519_verify tests ----

    #[test]
    fn host_registry_system_constant_time_eq_reports_match_and_mismatch() {
        let equal = HOST_REGISTRY
            .call(
                "sys_constant_time_eq",
                &[
                    RuntimeValue::String("discord-signature".to_string()),
                    RuntimeValue::String("discord-signature".to_string()),
                ],
            )
            .expect("sys_constant_time_eq should support equal strings");

        let different = HOST_REGISTRY
            .call(
                "sys_constant_time_eq",
                &[
                    RuntimeValue::String("discord-signature".to_string()),
                    RuntimeValue::String("discord-signature-x".to_string()),
                ],
            )
            .expect("sys_constant_time_eq should support mismatched strings");

        assert_eq!(equal, RuntimeValue::Bool(true));
        assert_eq!(different, RuntimeValue::Bool(false));
    }

    #[test]
    fn host_registry_system_discord_ed25519_verify_accepts_valid_signature() {
        let result = HOST_REGISTRY
            .call(
                "sys_discord_ed25519_verify",
                &[
                    RuntimeValue::String(
                        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
                            .to_string(),
                    ),
                    RuntimeValue::String(
                        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b"
                            .to_string(),
                    ),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::String(String::new()),
                ],
            )
            .expect("sys_discord_ed25519_verify should accept valid test-vector signatures");

        assert_eq!(result, RuntimeValue::Bool(true));
    }

    #[test]
    fn host_registry_system_discord_ed25519_verify_returns_false_for_invalid_signature() {
        let result = HOST_REGISTRY
            .call(
                "sys_discord_ed25519_verify",
                &[
                    RuntimeValue::String(
                        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
                            .to_string(),
                    ),
                    RuntimeValue::String(
                        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100c"
                            .to_string(),
                    ),
                    RuntimeValue::String(String::new()),
                    RuntimeValue::String(String::new()),
                ],
            )
            .expect("sys_discord_ed25519_verify should return false for invalid signatures");

        assert_eq!(result, RuntimeValue::Bool(false));
    }

    #[test]
    fn host_registry_system_discord_ed25519_verify_rejects_malformed_signature_hex() {
        let error = HOST_REGISTRY
            .call(
                "sys_discord_ed25519_verify",
                &[
                    RuntimeValue::String(
                        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a"
                            .to_string(),
                    ),
                    RuntimeValue::String("abcd".to_string()),
                    RuntimeValue::String("1700000000".to_string()),
                    RuntimeValue::String("{}".to_string()),
                ],
            )
            .expect_err("sys_discord_ed25519_verify should reject malformed signature hex");

        assert_eq!(
            error.to_string(),
            "host error: sys_discord_ed25519_verify signature_hex must be 128 hex chars, found 4"
        );
    }
}
