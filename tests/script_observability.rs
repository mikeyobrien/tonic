use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

#[test]
fn native_gates_with_observability_creates_correlated_task_and_step_events() {
    let fixture = setup_native_gates_fixture("script-observability-success");

    let output = Command::new("bash")
        .arg(fixture.root.join("scripts/native-gates.sh"))
        .current_dir(&fixture.root)
        .env("PATH", fixture.path_env())
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &fixture.obs_dir)
        .env("TONIC_STUB_LOG_DIR", fixture.root.join("stub-logs"))
        .output()
        .expect("native-gates should execute");

    assert!(
        output.status.success(),
        "expected native-gates to succeed, stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let latest = read_json(&fixture.obs_dir.join("latest.json"));
    let summary_path = PathBuf::from(
        latest["summary_path"]
            .as_str()
            .expect("latest summary path should be a string"),
    );
    let summary = read_json(&summary_path);
    let task_id = summary["task_id"]
        .as_str()
        .expect("task id should be recorded")
        .to_string();
    let run_id = summary["run_id"]
        .as_str()
        .expect("run id should be recorded")
        .to_string();

    assert_eq!(summary["tool"]["kind"], "script");
    assert_eq!(summary["tool"]["command"], "native-gates");
    assert_eq!(summary["status"], "ok");

    let events = fs::read_to_string(summary_path.parent().unwrap().join("events.jsonl"))
        .expect("root events should exist");
    assert!(events.contains("\"type\":\"step.started\""));
    assert!(events.contains("\"type\":\"step.finished\""));
    assert!(events.contains("cargo fmt --all -- --check"));
    assert!(events.contains("scripts/memory-bakeoff.sh --ci"));

    let emitted = summary["artifacts"]["emitted"]
        .as_array()
        .expect("artifacts emitted should be an array");
    let artifact_paths: Vec<&str> = emitted
        .iter()
        .map(|item| {
            item["path"]
                .as_str()
                .expect("artifact path should be a string")
        })
        .collect();
    assert!(artifact_paths
        .iter()
        .any(|path| path.ends_with("native-compiler-summary.json")));
    assert!(artifact_paths
        .iter()
        .any(|path| path.ends_with("native-compiled-summary.json")));
    assert!(artifact_paths
        .iter()
        .any(|path| path.contains("memory-bakeoff")));

    let task_index_path = fixture
        .obs_dir
        .join("tasks")
        .join(&task_id)
        .join("runs.jsonl");
    let task_entries = read_json_lines(&task_index_path);
    assert!(
        task_entries.len() >= 10,
        "expected root run plus child step runs, got {} entries",
        task_entries.len()
    );
    assert!(task_entries
        .iter()
        .any(|entry| entry["command"] == "native-gates"));

    let fmt_entry = task_entries
        .iter()
        .find(|entry| entry["command"] == "cargo fmt --all -- --check")
        .expect("fmt child run should be indexed");
    let fmt_run_id = fmt_entry["run_id"]
        .as_str()
        .expect("fmt run id should be a string");
    let fmt_summary = read_json(
        &fixture
            .obs_dir
            .join("runs")
            .join(fmt_run_id)
            .join("summary.json"),
    );
    assert_eq!(fmt_summary["parent_run_id"], run_id);
    assert_eq!(fmt_summary["task_id"], task_id);
    assert_eq!(fmt_summary["tool"]["kind"], "script-step");

    let differential_env =
        fs::read_to_string(fixture.root.join("stub-logs/differential-enforce.sh.env"))
            .expect("differential stub should capture env vars");
    assert!(differential_env.contains(&format!("task_id={task_id}")));
    assert!(differential_env.contains(&format!("parent_run_id={run_id}")));
    assert!(differential_env.contains("run_id=run_"));
}

#[test]
fn native_gates_preserves_success_when_observability_helper_fails() {
    let fixture = setup_native_gates_fixture("script-observability-fail-open");
    let blocked_obs_path = fixture.root.join("occupied-observability-path");
    fs::write(&blocked_obs_path, "not a directory").expect("blocked obs path should be created");

    let output = Command::new("bash")
        .arg(fixture.root.join("scripts/native-gates.sh"))
        .current_dir(&fixture.root)
        .env("PATH", fixture.path_env())
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &blocked_obs_path)
        .env("TONIC_STUB_LOG_DIR", fixture.root.join("stub-logs"))
        .output()
        .expect("native-gates should execute");

    assert!(
        output.status.success(),
        "observability helper failures must not change script success, stdout: {}, stderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("observability warning:"),
        "expected fail-open warning in stderr, got: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(fixture
        .artifact_dir
        .join("native-compiler-summary.json")
        .exists());
    assert!(fixture
        .artifact_dir
        .join("native-compiled-summary.json")
        .exists());
    assert!(
        !blocked_obs_path.join("latest.json").exists(),
        "blocked output path should not gain observability files"
    );
}

struct NativeGatesFixture {
    root: PathBuf,
    obs_dir: PathBuf,
    artifact_dir: PathBuf,
}

impl NativeGatesFixture {
    fn path_env(&self) -> String {
        let existing = std::env::var("PATH").unwrap_or_default();
        format!("{}:{}", self.root.join("bin").display(), existing)
    }
}

fn setup_native_gates_fixture(test_name: &str) -> NativeGatesFixture {
    let root = common::unique_fixture_root(test_name);
    let scripts_dir = root.join("scripts");
    let lib_dir = scripts_dir.join("lib");
    let bin_dir = root.join("bin");
    let obs_dir = root.join("observability");
    let artifact_dir = root.join(".tonic/native-gates");

    fs::create_dir_all(&lib_dir).expect("fixture should create lib dir");
    fs::create_dir_all(&bin_dir).expect("fixture should create bin dir");
    fs::create_dir_all(root.join("stub-logs")).expect("fixture should create stub log dir");

    copy_repo_script(
        "scripts/native-gates.sh",
        &scripts_dir.join("native-gates.sh"),
    );
    copy_repo_script(
        "scripts/lib/observability.sh",
        &lib_dir.join("observability.sh"),
    );
    copy_repo_script(
        "scripts/lib/observability_event.py",
        &lib_dir.join("observability_event.py"),
    );

    write_fake_cargo(&bin_dir.join("cargo"));
    write_stub_script(
        &scripts_dir.join("differential-enforce.sh"),
        "printf '%s\\n' 'differential ok'\n",
    );
    write_stub_script(
        &scripts_dir.join("bench-native-contract-enforce.sh"),
        "json_out=\"${TONIC_BENCH_JSON_OUT:?}\"\nmarkdown_out=\"${TONIC_BENCH_MARKDOWN_OUT:?}\"\nmkdir -p \"$(dirname \"$json_out\")\" \"$(dirname \"$markdown_out\")\"\nprintf '%s\\n' '{\"status\":\"pass\"}' > \"$json_out\"\nprintf '%s\\n' '# summary' > \"$markdown_out\"\n",
    );
    write_stub_script(
        &scripts_dir.join("native-regression-policy.sh"),
        "report_path=\"${1:?}\"\nif [[ ! -f \"$report_path\" ]]; then\n  printf 'missing report: %s\\n' \"$report_path\" >&2\n  exit 1\nfi\nprintf '%s\\n' 'verdict=pass'\n",
    );
    write_stub_script(
        &scripts_dir.join("memory-bakeoff.sh"),
        "artifact_dir=\"${TONIC_MEMORY_BAKEOFF_ARTIFACT_DIR:?}\"\nmkdir -p \"$artifact_dir\"\nprintf '%s\\n' 'scenario\tmode' > \"$artifact_dir/summary.tsv\"\nprintf '%s\\n' '# memory bakeoff' > \"$artifact_dir/summary.md\"\n",
    );

    NativeGatesFixture {
        root,
        obs_dir,
        artifact_dir,
    }
}

fn copy_repo_script(source: &str, destination: &Path) {
    fs::copy(repo_root().join(source), destination).expect("fixture should copy repo script");
    make_executable(destination);
}

fn write_fake_cargo(path: &Path) {
    let content = "#!/usr/bin/env bash\nset -euo pipefail\nprintf '%s\\n' \"$*\" >> \"${TONIC_STUB_LOG_DIR:?}/cargo.invocations.log\"\n";
    fs::write(path, content).expect("fixture should write fake cargo");
    make_executable(path);
}

fn write_stub_script(path: &Path, body: &str) {
    let content = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\nmkdir -p \"${{TONIC_STUB_LOG_DIR:?}}\"\nprintf 'run_id=%s task_id=%s parent_run_id=%s\\n' \\\n  \"${{TONIC_OBS_RUN_ID:-}}\" \\\n  \"${{TONIC_OBS_TASK_ID:-}}\" \\\n  \"${{TONIC_OBS_PARENT_RUN_ID:-}}\" > \"${{TONIC_STUB_LOG_DIR}}/$(basename \"$0\").env\"\n{}",
        body
    );
    fs::write(path, content).expect("fixture should write stub script");
    make_executable(path);
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("json file should be readable"))
        .expect("json should parse")
}

fn read_json_lines(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .expect("jsonl file should be readable")
        .lines()
        .map(|line| serde_json::from_str(line).expect("jsonl line should parse"))
        .collect()
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut perms = fs::metadata(path).expect("path should exist").permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).expect("should set executable bit");
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}
