mod common;

use predicates::prelude::*;

#[test]
fn install_missing_source_exits_64() {
    common::tonic_cmd(&["install"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("missing install source"));
}

#[test]
fn install_unknown_flag_exits_64() {
    common::tonic_cmd(&["install", "--bogus"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("unknown flag"));
}

#[test]
fn install_help_mentions_subcommands() {
    let stdout = common::tonic_success(&["install", "--help"]);
    assert!(stdout.contains("install"), "help should mention install");
    assert!(stdout.contains("--copy"), "help should mention --copy");
    assert!(stdout.contains("--force"), "help should mention --force");
}

#[test]
fn uninstall_missing_name_exits_64() {
    common::tonic_cmd(&["uninstall"])
        .assert()
        .code(64)
        .stderr(predicate::str::contains("missing package name"));
}

#[test]
fn uninstall_help_exits_ok() {
    common::tonic_cmd(&["uninstall", "--help"])
        .assert()
        .success();
}

#[test]
fn installed_help_exits_ok() {
    common::tonic_cmd(&["installed", "--help"])
        .assert()
        .success();
}

#[test]
fn installed_no_packages_exits_ok() {
    let (dir, home) = common::isolated_tonic_home("cli-contract-installed-empty");
    common::tonic_cmd(&["installed"])
        .env("TONIC_HOME", &home)
        .assert()
        .success()
        .stdout(predicate::str::contains("No packages installed"));
    let _ = std::fs::remove_dir_all(&dir);
}
