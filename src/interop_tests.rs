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

    let content = std::fs::read_to_string(&target_file).expect("report file should be readable");
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

    let streamed = HOST_REGISTRY
        .call(
            "sys_run",
            &[
                RuntimeValue::String(
                    "printf 'hello'; sleep 0.05; printf ' stderr' >&2".to_string(),
                ),
                RuntimeValue::Map(vec![(
                    RuntimeValue::Atom("stream".to_string()),
                    RuntimeValue::Bool(true),
                )]),
            ],
        )
        .expect("sys_run should accept streaming opts");

    assert_eq!(
        map_lookup(&streamed, "exit_code"),
        Some(&RuntimeValue::Int(0))
    );
    assert_eq!(
        map_lookup(&streamed, "output"),
        Some(&RuntimeValue::String("hello stderr".to_string()))
    );
    assert_eq!(
        map_lookup(&streamed, "timed_out"),
        Some(&RuntimeValue::Bool(false))
    );

    let timed_out = HOST_REGISTRY
        .call(
            "sys_run",
            &[
                RuntimeValue::String("printf 'hello'; sleep 1; printf 'tail'".to_string()),
                RuntimeValue::Map(vec![(
                    RuntimeValue::Atom("timeout_ms".to_string()),
                    RuntimeValue::Int(100),
                )]),
            ],
        )
        .expect("sys_run should return timeout result map");

    assert_eq!(
        map_lookup(&timed_out, "exit_code"),
        Some(&RuntimeValue::Int(124))
    );
    assert_eq!(
        map_lookup(&timed_out, "timed_out"),
        Some(&RuntimeValue::Bool(true))
    );
    assert_eq!(
        map_lookup(&timed_out, "output"),
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
fn host_registry_system_run_rejects_invalid_opts() {
    let bad_key = HOST_REGISTRY
        .call(
            "sys_run",
            &[
                RuntimeValue::String("printf 'hello'".to_string()),
                RuntimeValue::Map(vec![(
                    RuntimeValue::Atom("surprise".to_string()),
                    RuntimeValue::Bool(true),
                )]),
            ],
        )
        .expect_err("sys_run should reject unknown opts keys");

    assert_eq!(
        bad_key.to_string(),
        "host error: sys_run unsupported opts key: surprise"
    );

    let bad_stream = HOST_REGISTRY
        .call(
            "sys_run",
            &[
                RuntimeValue::String("printf 'hello'".to_string()),
                RuntimeValue::Map(vec![(
                    RuntimeValue::Atom("stream".to_string()),
                    RuntimeValue::String("yes".to_string()),
                )]),
            ],
        )
        .expect_err("sys_run should reject non-bool stream opts");

    assert_eq!(
        bad_stream.to_string(),
        "host error: sys_run opts.stream expects bool; found string"
    );

    let bad_timeout = HOST_REGISTRY
        .call(
            "sys_run",
            &[
                RuntimeValue::String("printf 'hello'".to_string()),
                RuntimeValue::Map(vec![(
                    RuntimeValue::Atom("timeout_ms".to_string()),
                    RuntimeValue::String("soon".to_string()),
                )]),
            ],
        )
        .expect_err("sys_run should reject non-int timeout opts");

    assert_eq!(
        bad_timeout.to_string(),
        "host error: sys_run opts.timeout_ms expects int; found string"
    );

    let negative_timeout = HOST_REGISTRY
        .call(
            "sys_run",
            &[
                RuntimeValue::String("printf 'hello'".to_string()),
                RuntimeValue::Map(vec![(
                    RuntimeValue::Atom("timeout_ms".to_string()),
                    RuntimeValue::Int(-1),
                )]),
            ],
        )
        .expect_err("sys_run should reject negative timeout opts");

    assert_eq!(
        negative_timeout.to_string(),
        "host error: sys_run opts.timeout_ms must be >= 0, found -1"
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
