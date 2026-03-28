use super::*;

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

#[cfg(feature = "network")]
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

#[cfg(feature = "network")]
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

#[cfg(feature = "network")]
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

#[cfg(feature = "network")]
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
