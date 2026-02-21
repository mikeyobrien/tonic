use assert_cmd::assert::OutputAssertExt;
use predicates::prelude::PredicateBooleanExt;
use predicates::str::contains;

#[test]
fn tonic_help_lists_v0_commands() {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));

    cmd.arg("--help").assert().success().stdout(
        contains("run")
            .and(contains("check"))
            .and(contains("test"))
            .and(contains("fmt"))
            .and(contains("cache"))
            .and(contains("verify")),
    );
}
