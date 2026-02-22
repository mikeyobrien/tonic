use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::{contains, starts_with};
use std::fs;
mod common;

#[test]
fn verify_run_auto_mode_emits_pass_fail_json() {
    let fixture_root = common::unique_fixture_root("verify-auto-json");
    let acceptance_dir = fixture_root.join("acceptance/features");

    fs::create_dir_all(&acceptance_dir)
        .expect("fixture setup should create acceptance/features directory");

    fs::write(
        fixture_root.join("acceptance/step-01.yaml"),
        "slice_id: step-01\nfeature_files:\n  - acceptance/features/step-01.feature\n",
    )
    .expect("fixture setup should write acceptance yaml");

    fs::write(
        fixture_root.join("acceptance/features/step-01.feature"),
        "Feature: Verify auto mode JSON\n\n  @auto\n  Scenario: auto-smoke\n    Given acceptance metadata exists\n",
    )
    .expect("fixture setup should write feature file");

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));

    cmd.current_dir(&fixture_root)
        .args(["verify", "run", "step-01", "--mode", "auto"])
        .assert()
        .success()
        .stderr("")
        .stdout(
            starts_with("{")
                .and(contains("\"slice_id\":\"step-01\""))
                .and(contains("\"mode\":\"auto\""))
                .and(contains("\"status\":\"pass\""))
                .and(contains("\"acceptance_file\":\"acceptance/step-01.yaml\"")),
        );
}
