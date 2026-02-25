use std::fs;
use std::path::PathBuf;
use toml::Value;

#[test]
fn native_compiled_suite_wiring_is_deterministic() {
    let manifest_path = repo_root().join("benchmarks/native-compiled-suite.toml");
    let manifest_str = fs::read_to_string(&manifest_path).expect("compiled suite should exist");

    let manifest: Value = toml::from_str(&manifest_str).expect("should parse as TOML");

    let contract = manifest
        .get("performance_contract")
        .expect("should have performance_contract");

    assert_eq!(
        contract.get("baseline_path").unwrap().as_str().unwrap(),
        "benchmarks/native-compiler-baselines.json",
        "must point to shared baselines"
    );
    assert_eq!(
        contract.get("candidate_target").unwrap().as_str().unwrap(),
        "compiled",
        "must measure compiled target"
    );

    let ref_targets = contract
        .get("reference_targets")
        .unwrap()
        .as_array()
        .unwrap();
    assert!(
        ref_targets
            .iter()
            .any(|t| t.as_str().unwrap() == "tonic_compiled_baseline"),
        "must reference compiled baseline"
    );
}

#[test]
fn ci_native_gates_enforces_dual_strict_flow() {
    let workflow_path = repo_root().join(".github/workflows/native-gates.yml");
    let workflow = fs::read_to_string(&workflow_path).expect("workflow should exist");

    // Check interpreter flow
    assert!(
        workflow.contains("TONIC_BENCH_JSON_OUT: .tonic/native-gates/native-compiler-summary.json")
    );
    assert!(workflow.contains("./scripts/bench-native-contract-enforce.sh"));
    assert!(workflow.contains("./scripts/native-regression-policy.sh .tonic/native-gates/native-compiler-summary.json --mode strict"));

    // Check compiled flow
    assert!(
        workflow.contains("TONIC_BENCH_JSON_OUT: .tonic/native-gates/native-compiled-summary.json")
    );
    assert!(workflow.contains("TONIC_BENCH_TARGET_NAME: \"compiled\""));
    assert!(workflow.contains(
        "./scripts/bench-native-contract-enforce.sh benchmarks/native-compiled-suite.toml"
    ));
    assert!(workflow.contains("./scripts/native-regression-policy.sh .tonic/native-gates/native-compiled-summary.json --mode strict"));
}

#[test]
fn docs_cover_dual_strict_flow() {
    let docs_path = repo_root().join("docs/native-regression-policy.md");
    let docs = fs::read_to_string(&docs_path).expect("docs should exist");

    assert!(docs.contains("TONIC_BENCH_TARGET_NAME=compiled"));
    assert!(docs.contains("benchmarks/native-compiled-suite.toml"));
    assert!(docs.contains("native-compiled-summary.json"));
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
