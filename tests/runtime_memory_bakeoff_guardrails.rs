use std::fs;
use std::path::PathBuf;

#[test]
fn native_gates_workflow_runs_memory_bakeoff_guardrail() {
    let workflow_path = repo_root().join(".github/workflows/native-gates.yml");
    let workflow = fs::read_to_string(&workflow_path).expect("workflow should exist");

    assert!(
        workflow.contains("./scripts/memory-bakeoff.sh --ci"),
        "native-gates workflow should run memory bakeoff guardrail script"
    );
}

#[test]
fn memory_bakeoff_report_is_committed_with_default_selection() {
    let report_path = repo_root().join("docs/runtime-memory-bakeoff.md");
    let report = fs::read_to_string(&report_path).expect("bakeoff report should exist");

    assert!(
        report.contains("## Results (baseline vs RC vs trace)"),
        "report should include bakeoff results table"
    );
    assert!(
        report.contains("## Default strategy decision"),
        "report should include explicit default strategy decision"
    );
    assert!(
        report.contains("TONIC_MEMORY_MODE=append_only"),
        "report should include rollback path"
    );
}

#[test]
fn native_runtime_docs_describe_default_and_rollback_modes() {
    let docs_path = repo_root().join("docs/native-runtime.md");
    let docs = fs::read_to_string(&docs_path).expect("native runtime docs should exist");

    assert!(
        docs.contains("Default mode: tracing mark/sweep"),
        "docs should explicitly state the default memory mode"
    );
    assert!(
        docs.contains("TONIC_MEMORY_MODE=append_only"),
        "docs should document append-only rollback mode"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
