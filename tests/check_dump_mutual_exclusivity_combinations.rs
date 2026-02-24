use std::path::PathBuf;
mod common;

fn write_minimal_fixture(fixture_root: &std::path::Path) -> PathBuf {
    std::fs::create_dir_all(fixture_root).expect("fixture setup should create directory");
    let source_path = fixture_root.join("test.tn");
    std::fs::write(&source_path, "defmodule A do\nend\n")
        .expect("fixture setup should write source");
    source_path
}

fn check_with_flags(source: &str, flags: &[&str]) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .args(
            std::iter::once("check")
                .chain(std::iter::once(source))
                .chain(flags.iter().copied()),
        )
        .output()
        .expect("check command should execute")
}

const EXCLUSIVITY_MSG: &str =
    "--dump-tokens, --dump-ast, --dump-ir, and --dump-mir cannot be used together";

#[test]
fn check_rejects_dump_tokens_and_dump_ast_together() {
    let fixture_root = common::unique_fixture_root("dump-tokens-ast-exclusivity");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(
        source_path.to_str().unwrap(),
        &["--dump-tokens", "--dump-ast"],
    );

    assert!(
        !output.status.success(),
        "expected failure when --dump-tokens and --dump-ast are combined"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(EXCLUSIVITY_MSG),
        "expected deterministic usage error, got: {stderr}"
    );
}

#[test]
fn check_rejects_dump_tokens_and_dump_ir_together() {
    let fixture_root = common::unique_fixture_root("dump-tokens-ir-exclusivity");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(
        source_path.to_str().unwrap(),
        &["--dump-tokens", "--dump-ir"],
    );

    assert!(
        !output.status.success(),
        "expected failure when --dump-tokens and --dump-ir are combined"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(EXCLUSIVITY_MSG),
        "expected deterministic usage error, got: {stderr}"
    );
}

#[test]
fn check_rejects_all_three_dump_flags_together() {
    let fixture_root = common::unique_fixture_root("dump-all-three-exclusivity");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(
        source_path.to_str().unwrap(),
        &["--dump-tokens", "--dump-ast", "--dump-ir"],
    );

    assert!(
        !output.status.success(),
        "expected failure when all three dump flags are combined"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(EXCLUSIVITY_MSG),
        "expected deterministic usage error, got: {stderr}"
    );
}

#[test]
fn check_dump_tokens_alone_succeeds() {
    let fixture_root = common::unique_fixture_root("dump-tokens-solo");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(source_path.to_str().unwrap(), &["--dump-tokens"]);

    assert!(
        output.status.success(),
        "expected --dump-tokens alone to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn check_dump_ast_alone_succeeds() {
    let fixture_root = common::unique_fixture_root("dump-ast-solo");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(source_path.to_str().unwrap(), &["--dump-ast"]);

    assert!(
        output.status.success(),
        "expected --dump-ast alone to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let _parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--dump-ast output should be valid JSON");
}

#[test]
fn check_dump_ir_alone_succeeds() {
    let fixture_root = common::unique_fixture_root("dump-ir-solo");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(source_path.to_str().unwrap(), &["--dump-ir"]);

    assert!(
        output.status.success(),
        "expected --dump-ir alone to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let _parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--dump-ir output should be valid JSON");
}

#[test]
fn check_rejects_dump_ir_and_dump_mir_together() {
    let fixture_root = common::unique_fixture_root("dump-ir-mir-exclusivity");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(source_path.to_str().unwrap(), &["--dump-ir", "--dump-mir"]);

    assert!(
        !output.status.success(),
        "expected failure when --dump-ir and --dump-mir are combined"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains(EXCLUSIVITY_MSG),
        "expected deterministic usage error, got: {stderr}"
    );
}

#[test]
fn check_dump_mir_alone_succeeds() {
    let fixture_root = common::unique_fixture_root("dump-mir-solo");
    let source_path = write_minimal_fixture(&fixture_root);
    let output = check_with_flags(source_path.to_str().unwrap(), &["--dump-mir"]);

    assert!(
        output.status.success(),
        "expected --dump-mir alone to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let _parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--dump-mir output should be valid JSON");
}
