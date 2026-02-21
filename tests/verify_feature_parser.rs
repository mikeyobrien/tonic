use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;
use std::fs;
use std::path::PathBuf;

#[test]
fn verify_run_reports_feature_scenario_ids_and_tags() {
    let fixture_root = unique_fixture_root("feature-parser");
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
        "Feature: Verify slice metadata\n\n  @auto\n  Scenario: auto-smoke\n    Given tonic verify can load acceptance metadata\n\n  @agent-manual\n  Scenario: agent-review\n    Given an agent validates diagnostics manually\n\n  @human-manual\n  Scenario: human-ux\n    Given a human validates terminal ergonomics\n",
    )
    .expect("fixture setup should write feature file");

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));

    cmd.current_dir(&fixture_root)
        .args(["verify", "run", "step-01", "--mode", "manual"])
        .assert()
        .success()
        .stdout(
            contains("auto-smoke")
                .and(contains("agent-review"))
                .and(contains("human-ux"))
                .and(contains("@auto"))
                .and(contains("@agent-manual"))
                .and(contains("@human-manual")),
        );
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
