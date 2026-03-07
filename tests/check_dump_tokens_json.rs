use serde_json::json;
use std::fs;
mod common;

#[test]
fn check_dump_tokens_json_reports_kind_lexeme_and_spans() {
    let fixture_root = common::unique_fixture_root("check-dump-tokens-json");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(examples_dir.join("token_dump.tn"), "a + b")
        .expect("fixture setup should write token dump source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args([
            "check",
            "examples/token_dump.tn",
            "--dump-tokens",
            "--format",
            "json",
        ])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let actual: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("--dump-tokens --format json should emit JSON");
    let expected = json!([
        {"kind": "IDENT", "lexeme": "a", "span_start": 0, "span_end": 1},
        {"kind": "PLUS", "lexeme": "", "span_start": 2, "span_end": 3},
        {"kind": "IDENT", "lexeme": "b", "span_start": 4, "span_end": 5},
        {"kind": "EOF", "lexeme": "", "span_start": 5, "span_end": 5}
    ]);

    assert_eq!(actual, expected);
}

#[test]
fn check_rejects_format_without_dump_tokens() {
    let fixture_root = common::unique_fixture_root("check-dump-format-without-tokens");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(examples_dir.join("token_dump.tn"), "a + b")
        .expect("fixture setup should write token dump source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/token_dump.tn", "--format", "json"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected usage failure when --format is used without --dump-tokens"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("--format is only supported with --dump-tokens"),
        "expected deterministic usage error, got: {stderr}"
    );
}
