use serde_json::Value;
use std::fs;
use std::path::PathBuf;

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

#[test]
fn verify_run_mixed_mode_fails_when_manual_evidence_contains_invalid_json() {
    let fixture_root = unique_fixture_root("verify-manual-evidence-invalid-json");
    let acceptance_dir = fixture_root.join("acceptance/features");
    let evidence_dir = fixture_root.join("acceptance/evidence");

    fs::create_dir_all(&acceptance_dir)
        .expect("fixture setup should create acceptance/features directory");
    fs::create_dir_all(&evidence_dir)
        .expect("fixture setup should create acceptance/evidence directory");

    fs::write(
        fixture_root.join("acceptance/step-13.yaml"),
        "slice_id: step-13\n\
         feature_files:\n  - acceptance/features/step-13.feature\n\
         manual_evidence:\n  mixed:\n    - acceptance/evidence/agent-review.json\n",
    )
    .expect("fixture setup should write acceptance yaml");

    fs::write(
        fixture_root.join("acceptance/features/step-13.feature"),
        "Feature: Verify manual evidence invalid json\n\n  \
         @auto\n  Scenario: auto-smoke\n    Given automated checks exist\n\n  \
         @agent-manual\n  Scenario: agent-review\n    Given an agent must attach manual evidence\n",
    )
    .expect("fixture setup should write feature file");

    // Write evidence file with invalid JSON content
    fs::write(
        fixture_root.join("acceptance/evidence/agent-review.json"),
        "this is not valid json {{{",
    )
    .expect("fixture setup should write invalid evidence file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["verify", "run", "step-13", "--mode", "mixed"])
        .output()
        .expect("verify command should execute");

    assert!(
        !output.status.success(),
        "expected verify to fail when manual evidence contains invalid JSON, got status {:?} stdout: {} stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("verify stdout should be utf8");
    let report: Value =
        serde_json::from_str(stdout.trim()).expect("verify output should be valid JSON");

    assert_eq!(report["status"], "fail");
    assert_eq!(report["manual_evidence"]["status"], "invalid_payload");
    assert_eq!(
        report["manual_evidence"]["required"][0],
        "acceptance/evidence/agent-review.json"
    );
    assert_eq!(
        report["manual_evidence"]["invalid"][0],
        "acceptance/evidence/agent-review.json"
    );
    // missing list must be empty — the file exists, it is only malformed
    assert!(
        report["manual_evidence"]["missing"]
            .as_array()
            .is_some_and(|v| v.is_empty()),
        "missing list should be empty when evidence file exists but has invalid JSON"
    );
}

#[test]
fn verify_run_mixed_mode_passes_when_manual_evidence_is_valid_json() {
    let fixture_root = unique_fixture_root("verify-manual-evidence-valid-json");
    let acceptance_dir = fixture_root.join("acceptance/features");
    let evidence_dir = fixture_root.join("acceptance/evidence");

    fs::create_dir_all(&acceptance_dir)
        .expect("fixture setup should create acceptance/features directory");
    fs::create_dir_all(&evidence_dir)
        .expect("fixture setup should create acceptance/evidence directory");

    fs::write(
        fixture_root.join("acceptance/step-14.yaml"),
        "slice_id: step-14\n\
         feature_files:\n  - acceptance/features/step-14.feature\n\
         manual_evidence:\n  mixed:\n    - acceptance/evidence/agent-review.json\n",
    )
    .expect("fixture setup should write acceptance yaml");

    fs::write(
        fixture_root.join("acceptance/features/step-14.feature"),
        "Feature: Verify manual evidence valid json\n\n  \
         @auto\n  Scenario: auto-smoke\n    Given automated checks exist\n\n  \
         @agent-manual\n  Scenario: agent-review\n    Given an agent must attach manual evidence\n",
    )
    .expect("fixture setup should write feature file");

    // Write evidence file with valid JSON content
    fs::write(
        fixture_root.join("acceptance/evidence/agent-review.json"),
        r#"{"status": "pass", "notes": "all scenarios reviewed"}"#,
    )
    .expect("fixture setup should write valid evidence file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["verify", "run", "step-14", "--mode", "mixed"])
        .output()
        .expect("verify command should execute");

    assert!(
        output.status.success(),
        "expected verify to pass when manual evidence is valid JSON, got status {:?} stdout: {} stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("verify stdout should be utf8");
    let report: Value =
        serde_json::from_str(stdout.trim()).expect("verify output should be valid JSON");

    assert_eq!(report["status"], "pass");
    assert_eq!(report["manual_evidence"]["status"], "pass");
    assert!(
        report["manual_evidence"]["missing"]
            .as_array()
            .is_some_and(|v| v.is_empty()),
        "missing list should be empty"
    );
    assert!(
        report["manual_evidence"]["invalid"]
            .as_array()
            .is_some_and(|v| v.is_empty()),
        "invalid list should be empty"
    );
}
