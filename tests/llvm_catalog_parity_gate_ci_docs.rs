use std::fs;
use std::path::PathBuf;

#[test]
fn parity_enforce_script_exists_and_enforces_catalog_gate() {
    let script_path = repo_root().join("scripts/llvm-catalog-parity-enforce.sh");
    let script = fs::read_to_string(&script_path).expect("parity gate script should exist");

    assert!(
        script.contains("--enforce"),
        "script must run llvm_catalog_parity in enforce mode"
    );
    assert!(
        script.contains("TONIC_PARITY_REPORT_JSON"),
        "script should expose JSON report output override"
    );
    assert!(
        script.contains("TONIC_PARITY_REPORT_MD"),
        "script should expose markdown report output override"
    );
}

#[test]
fn native_gates_workflow_runs_parity_gate_and_uploads_artifacts() {
    let workflow_path = repo_root().join(".github/workflows/native-gates.yml");
    let workflow = fs::read_to_string(&workflow_path).expect("workflow should be readable");

    assert!(
        workflow.contains("scripts/llvm-catalog-parity-enforce.sh"),
        "native-gates workflow must execute parity gate script"
    );
    assert!(
        workflow.contains("Upload LLVM parity artifacts"),
        "native-gates workflow must upload parity artifacts"
    );
    assert!(
        workflow.contains(".tonic/parity/"),
        "native-gates workflow must publish parity report directory"
    );
}

#[test]
fn differential_docs_cover_local_llvm_parity_gate_usage() {
    let docs_path = repo_root().join("docs/differential-testing.md");
    let docs = fs::read_to_string(&docs_path).expect("differential docs should be readable");

    assert!(
        docs.contains("scripts/llvm-catalog-parity-enforce.sh"),
        "docs must explain how to run parity gate locally"
    );
    assert!(
        docs.contains(".tonic/parity/llvm-catalog-parity.json"),
        "docs must document parity JSON report location"
    );
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
