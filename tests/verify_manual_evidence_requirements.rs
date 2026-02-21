use serde_json::Value;
use std::fs;
use std::path::PathBuf;

#[test]
fn verify_run_mixed_mode_fails_when_required_manual_evidence_json_is_missing() {
    let fixture_root = write_verify_fixture("verify-manual-evidence-missing");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["verify", "run", "step-13", "--mode", "mixed"])
        .output()
        .expect("verify command should execute");

    assert!(
        !output.status.success(),
        "expected verify to fail when required manual evidence is missing, got status {:?} with stdout: {} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("verify stdout should be utf8");
    let report: Value = serde_json::from_str(stdout.trim()).expect("verify output should be json");

    assert_eq!(report["status"], "fail");
    assert_eq!(report["manual_evidence"]["status"], "missing_required");
    assert_eq!(
        report["manual_evidence"]["required"][0],
        "acceptance/evidence/agent-review.json"
    );
    assert_eq!(
        report["manual_evidence"]["missing"][0],
        "acceptance/evidence/agent-review.json"
    );
}

fn write_verify_fixture(test_name: &str) -> PathBuf {
    let fixture_root = unique_fixture_root(test_name);
    let acceptance_dir = fixture_root.join("acceptance/features");

    fs::create_dir_all(&acceptance_dir)
        .expect("fixture setup should create acceptance/features directory");

    fs::write(
        fixture_root.join("acceptance/step-13.yaml"),
        "slice_id: step-13\nfeature_files:\n  - acceptance/features/step-13.feature\nmanual_evidence:\n  mixed:\n    - acceptance/evidence/agent-review.json\n",
    )
    .expect("fixture setup should write acceptance yaml");

    fs::write(
        fixture_root.join("acceptance/features/step-13.feature"),
        "Feature: Verify manual evidence requirements\n\n  @auto\n  Scenario: auto-smoke\n    Given automated checks exist\n\n  @agent-manual\n  Scenario: agent-review\n    Given an agent must attach manual evidence\n",
    )
    .expect("fixture setup should write feature file");

    fixture_root
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}
