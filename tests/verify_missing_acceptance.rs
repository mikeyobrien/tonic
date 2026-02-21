use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn verify_run_fails_when_acceptance_yaml_is_missing() {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));

    cmd.args(["verify", "run", "step-01", "--mode", "auto"])
        .assert()
        .failure()
        .stderr(contains("missing acceptance file").and(contains("acceptance/step-01.yaml")));
}
