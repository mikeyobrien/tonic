use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

mod common;

#[test]
fn tonic_observability_skill_has_valid_frontmatter_and_decision_guidance() {
    let skill_path = repo_root().join(".agents/skills/tonic-observability/SKILL.md");
    let skill = fs::read_to_string(&skill_path).expect("tonic observability skill should exist");

    assert!(skill.starts_with("---\nname: tonic-observability\n"));
    assert!(
        skill.contains("description:"),
        "skill frontmatter should include a description"
    );
    assert!(
        skill.contains("debugging compiler/runtime failures"),
        "skill should recommend telemetry for debugging work"
    );
    assert!(
        skill.contains("native gates") && skill.contains("benchmark") && skill.contains("memory"),
        "skill should cover the main telemetry-heavy workflows"
    );
    assert!(
        skill.contains("Keep observability off"),
        "skill should explain when not to enable telemetry"
    );
}

#[test]
fn observability_docs_cover_env_layout_and_phase_scoping() {
    let docs_path = repo_root().join("docs/observability.md");
    let docs = fs::read_to_string(&docs_path).expect("observability docs should exist");

    for env_var in [
        "TONIC_OBS_ENABLE",
        "TONIC_OBS_DIR",
        "TONIC_OBS_RUN_ID",
        "TONIC_OBS_TASK_ID",
        "TONIC_OBS_PARENT_RUN_ID",
    ] {
        assert!(docs.contains(env_var), "docs should mention {env_var}");
    }

    for layout_entry in [
        ".tonic/observability/",
        "runs/<run-id>/summary.json",
        "events.jsonl",
        "artifacts.json",
        "tasks/<task-id>/runs.jsonl",
        "latest.json",
    ] {
        assert!(
            docs.contains(layout_entry),
            "docs should mention {layout_entry}"
        );
    }

    assert!(
        docs.contains("Phase 1") && docs.contains("Phase 2"),
        "docs should distinguish current scope from later work"
    );
    assert!(
        docs.contains("optional") && docs.contains("OTLP"),
        "docs should label export/UI work as optional future work"
    );
}

#[test]
fn observed_check_workflow_matches_documented_bundle_paths() {
    let fixture_root = common::unique_fixture_root("observability-docs-workflow");
    let obs_dir = fixture_root.join("observability");
    fs::write(
        fixture_root.join("demo.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("fixture should write source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .env("TONIC_OBS_ENABLE", "1")
        .env("TONIC_OBS_DIR", &obs_dir)
        .args(["check", "demo.tn"])
        .output()
        .expect("check command should execute");

    assert!(
        output.status.success(),
        "expected documented check workflow to succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let latest = read_json(&obs_dir.join("latest.json"));
    let summary_path = PathBuf::from(
        latest["summary_path"]
            .as_str()
            .expect("latest summary path should be a string"),
    );
    assert!(summary_path.starts_with(&obs_dir));
    assert!(summary_path.ends_with(Path::new("summary.json")));

    let summary = read_json(&summary_path);
    assert_eq!(summary["tool"]["command"], "check");
    assert_eq!(summary["target_path"], "demo.tn");

    let docs = fs::read_to_string(repo_root().join("docs/observability.md"))
        .expect("observability docs should exist");
    assert!(docs.contains("TONIC_OBS_ENABLE=1 cargo run --bin tonic -- check"));
    assert!(docs.contains("latest.json"));
    assert!(docs.contains("summary.json"));
}

#[test]
fn readme_points_to_observability_docs() {
    let readme = fs::read_to_string(repo_root().join("README.md")).expect("README should exist");

    assert!(readme.contains("docs/observability.md"));
    assert!(readme.contains("TONIC_OBS_ENABLE=1"));
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(path).expect("json file should be readable"))
        .expect("json should parse")
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
