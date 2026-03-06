use super::{
    ObservabilityError, ObservabilityRun, OBS_DIR_ENV, OBS_ENABLE_ENV, OBS_PARENT_RUN_ID_ENV,
    OBS_RUN_ID_ENV, OBS_TASK_ID_ENV,
};
use serde_json::Value;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

struct EnvGuard {
    saved: Vec<(&'static str, Option<OsString>)>,
}

impl EnvGuard {
    fn new() -> Self {
        let keys = [
            OBS_ENABLE_ENV,
            OBS_DIR_ENV,
            OBS_RUN_ID_ENV,
            OBS_TASK_ID_ENV,
            OBS_PARENT_RUN_ID_ENV,
        ];
        let saved = keys
            .into_iter()
            .map(|key| (key, std::env::var_os(key)))
            .collect();
        Self { saved }
    }

    fn set(&self, key: &'static str, value: &str) {
        std::env::set_var(key, value);
    }

    fn remove(&self, key: &'static str) {
        std::env::remove_var(key);
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        for (key, value) in self.saved.drain(..) {
            match value {
                Some(value) => std::env::set_var(key, value),
                None => std::env::remove_var(key),
            }
        }
    }
}

fn unique_temp_dir(label: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "tonic-observability-{label}-{nanos}-{}",
        std::process::id()
    ));
    fs::create_dir_all(&dir).expect("temp dir should be created");
    dir
}

#[test]
fn observability_is_disabled_without_enable_flag() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let env = EnvGuard::new();
    env.remove(OBS_ENABLE_ENV);

    assert!(ObservabilityRun::from_env("run", &["run".to_string()], Path::new(".")).is_none());
}

#[test]
fn observed_run_bundle_writes_expected_files_and_schema() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let env = EnvGuard::new();
    let cwd = unique_temp_dir("bundle");
    let obs_dir = cwd.join("obs-root");
    let artifact_path = cwd.join("demo.out");
    fs::write(&artifact_path, "artifact").unwrap();
    env.set(OBS_ENABLE_ENV, "1");
    env.set(OBS_DIR_ENV, obs_dir.to_str().unwrap());
    env.set(OBS_RUN_ID_ENV, "run-test-001");

    let argv = vec!["run".to_string(), "demo.tn".to_string()];
    let mut run = ObservabilityRun::from_env("run", &argv, &cwd).expect("run should be enabled");
    let answer = run.phase("frontend.parse_ast", || 42);
    assert_eq!(answer, 42);
    run.record_artifact("native-executable", &artifact_path);
    let warnings = run.finish_ok(0);
    assert!(
        warnings.is_empty(),
        "expected no warnings, got {warnings:?}"
    );

    let bundle_dir = obs_dir.join("runs/run-test-001");
    assert!(bundle_dir.join("summary.json").exists());
    assert!(bundle_dir.join("events.jsonl").exists());
    assert!(bundle_dir.join("artifacts.json").exists());
    assert!(obs_dir.join("latest.json").exists());

    let summary: Value =
        serde_json::from_str(&fs::read_to_string(bundle_dir.join("summary.json")).unwrap())
            .unwrap();
    assert_eq!(summary["schema_name"], "tonic.observability.run");
    assert_eq!(summary["schema_version"], 1);
    assert_eq!(summary["run_id"], "run-test-001");
    assert_eq!(summary["status"], "ok");
    assert_eq!(summary["exit_code"], 0);
    assert_eq!(summary["argv"], serde_json::json!(["run", "demo.tn"]));
    assert_eq!(summary["target_path"], "demo.tn");
    assert_eq!(summary["tool"]["command"], "run");
    assert_eq!(summary["phases"].as_array().unwrap().len(), 1);
    assert_eq!(summary["artifacts"]["emitted"].as_array().unwrap().len(), 1);
    assert!(summary["legacy_signals"].is_object());

    let events = fs::read_to_string(bundle_dir.join("events.jsonl")).unwrap();
    assert!(events.contains("run.started"));
    assert!(events.contains("phase.finished"));
    assert!(events.contains("artifact.written"));
    assert!(events.contains("run.finished"));

    let artifacts: Value =
        serde_json::from_str(&fs::read_to_string(bundle_dir.join("artifacts.json")).unwrap())
            .unwrap();
    assert_eq!(artifacts["schema_name"], "tonic.observability.artifacts");
    assert_eq!(artifacts["items"].as_array().unwrap().len(), 1);
    assert_eq!(artifacts["items"][0]["bytes"], 8);
}

#[test]
fn task_and_parent_identity_are_preserved_in_summary_and_task_index() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let env = EnvGuard::new();
    let cwd = unique_temp_dir("task-index");
    let obs_dir = cwd.join("obs-root");
    env.set(OBS_ENABLE_ENV, "1");
    env.set(OBS_DIR_ENV, obs_dir.to_str().unwrap());
    env.set(OBS_RUN_ID_ENV, "run-test-002");
    env.set(OBS_TASK_ID_ENV, "task-abc");
    env.set(OBS_PARENT_RUN_ID_ENV, "parent-123");

    let argv = vec!["compile".to_string(), "demo.tn".to_string()];
    let mut run =
        ObservabilityRun::from_env("compile", &argv, &cwd).expect("run should be enabled");
    let warnings = run.finish_ok(0);
    assert!(
        warnings.is_empty(),
        "expected no warnings, got {warnings:?}"
    );

    let summary: Value = serde_json::from_str(
        &fs::read_to_string(obs_dir.join("runs/run-test-002/summary.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(summary["task_id"], "task-abc");
    assert_eq!(summary["parent_run_id"], "parent-123");

    let task_index = fs::read_to_string(obs_dir.join("tasks/task-abc/runs.jsonl")).unwrap();
    assert!(task_index.contains("run-test-002"));
    assert!(task_index.contains("compile"));
    assert!(task_index.contains("tonic.observability.task-run"));
}

#[test]
fn recorded_error_is_serialized_in_summary() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let env = EnvGuard::new();
    let cwd = unique_temp_dir("error-summary");
    let obs_dir = cwd.join("obs-root");
    env.set(OBS_ENABLE_ENV, "1");
    env.set(OBS_DIR_ENV, obs_dir.to_str().unwrap());
    env.set(OBS_RUN_ID_ENV, "run-test-err");

    let argv = vec!["check".to_string(), "demo.tn".to_string()];
    let mut run = ObservabilityRun::from_env("check", &argv, &cwd).expect("run should be enabled");
    run.record_error(ObservabilityError {
        kind: "typing_error".to_string(),
        diagnostic_code: Some("E2001".to_string()),
        phase: Some("frontend.infer_types".to_string()),
        message: "type mismatch".to_string(),
        source: None,
    });
    let warnings = run.finish_ok(1);
    assert!(
        warnings.is_empty(),
        "expected no warnings, got {warnings:?}"
    );

    let summary: Value = serde_json::from_str(
        &fs::read_to_string(obs_dir.join("runs/run-test-err/summary.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(summary["error"]["kind"], "typing_error");
    assert_eq!(summary["error"]["diagnostic_code"], "E2001");
    assert_eq!(summary["error"]["phase"], "frontend.infer_types");
}

#[test]
fn unwritable_output_path_returns_warning_without_failing_finish() {
    let _lock = env_lock()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let env = EnvGuard::new();
    let cwd = unique_temp_dir("fail-open");
    let file_path = cwd.join("not-a-directory");
    fs::write(&file_path, "occupied").unwrap();
    env.set(OBS_ENABLE_ENV, "1");
    env.set(OBS_DIR_ENV, file_path.to_str().unwrap());
    env.set(OBS_RUN_ID_ENV, "run-test-003");

    let argv = vec!["check".to_string(), "demo.tn".to_string()];
    let mut run = ObservabilityRun::from_env("check", &argv, &cwd).expect("run should be enabled");
    let warnings = run.finish_error(
        1,
        ObservabilityError {
            kind: "usage_error".to_string(),
            diagnostic_code: None,
            phase: Some("frontend.parse_ast".to_string()),
            message: "boom".to_string(),
            source: None,
        },
    );
    assert!(!warnings.is_empty(), "expected warning for unwritable path");
    assert!(warnings[0].contains("failed to create observability bundle directory"));
}
