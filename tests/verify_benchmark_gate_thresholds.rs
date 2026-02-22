use serde_json::Value;
use std::fs;
use std::path::PathBuf;
mod common;

#[test]
fn verify_run_fails_when_benchmark_thresholds_are_exceeded() {
    let fixture_root = write_verify_fixture("verify-benchmark-threshold-gate");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["verify", "run", "step-13", "--mode", "auto"])
        .output()
        .expect("verify command should execute");

    assert!(
        !output.status.success(),
        "expected verify to fail when benchmark thresholds are exceeded, got status {:?} with stdout: {} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("verify stdout should be utf8");
    let report: Value = serde_json::from_str(stdout.trim()).expect("verify output should be json");

    assert_eq!(report["status"], "fail");
    assert_eq!(report["benchmark"]["status"], "threshold_exceeded");
    assert_eq!(report["benchmark"]["thresholds"]["cold_start_p50_ms"], 50);
    assert_eq!(report["benchmark"]["thresholds"]["warm_start_p50_ms"], 10);
    assert_eq!(report["benchmark"]["thresholds"]["idle_rss_mb"], 30);
    assert_eq!(report["benchmark"]["measured"]["cold_start_p50_ms"], 74);
    assert_eq!(report["benchmark"]["measured"]["warm_start_p50_ms"], 15);
    assert_eq!(report["benchmark"]["measured"]["idle_rss_mb"], 42);
}

fn write_verify_fixture(test_name: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let acceptance_dir = fixture_root.join("acceptance/features");

    fs::create_dir_all(&acceptance_dir)
        .expect("fixture setup should create acceptance/features directory");

    fs::write(
        fixture_root.join("acceptance/step-13.yaml"),
        "slice_id: step-13\nfeature_files:\n  - acceptance/features/step-13.feature\nbenchmark_metrics:\n  cold_start_p50_ms: 74\n  warm_start_p50_ms: 15\n  idle_rss_mb: 42\n",
    )
    .expect("fixture setup should write acceptance yaml");

    fs::write(
        fixture_root.join("acceptance/features/step-13.feature"),
        "Feature: Verify benchmark gate\n\n  @auto\n  Scenario: benchmark-thresholds\n    Given benchmark measurements exist\n",
    )
    .expect("fixture setup should write feature file");

    fixture_root
}
