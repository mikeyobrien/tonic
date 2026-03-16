use super::*;

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
