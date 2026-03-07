use std::fs;
use std::process::Command;

mod common;

#[test]
fn check_self_hosted_lexer_scaffold_project_succeeds() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&repo_root)
        .args(["check", "examples/apps/self_hosted_lexer"])
        .output()
        .expect("check command should execute");

    assert!(
        output.status.success(),
        "expected self_hosted_lexer scaffold check success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("check: ok"),
        "expected check success marker, got: {stdout}"
    );
}

#[test]
fn run_self_hosted_lexer_scaffold_emits_keyword_tokens_with_spans() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = common::unique_fixture_root("run-self-hosted-lexer-keywords");
    let source_path = fixture_root.join("keywords.tn");
    fs::write(&source_path, "defmodule Demo do\nend\n")
        .expect("fixture setup should write keyword source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&repo_root)
        .args([
            "run",
            "examples/apps/self_hosted_lexer",
            source_path.to_str().expect("source path should be utf8"),
        ])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected self_hosted_lexer run success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    for needle in [
        "%{:source_path => \"",
        source_path.to_str().expect("source path should be utf8"),
        ":source_length => 22",
        ":tokens => [%{:kind => \"DEFMODULE\", :lexeme => \"defmodule\", :span_start => 0, :span_end => 9}, %{:kind => \"IDENT\", :lexeme => \"Demo\", :span_start => 10, :span_end => 14}, %{:kind => \"DO\", :lexeme => \"do\", :span_start => 15, :span_end => 17}, %{:kind => \"END\", :lexeme => \"end\", :span_start => 18, :span_end => 21}, %{:kind => \"EOF\", :lexeme => \"\", :span_start => 22, :span_end => 22}]",
    ] {
        assert!(
            stdout.contains(needle),
            "expected self-hosted lexer output to contain '{needle}', got: {stdout}"
        );
    }
}

#[test]
fn run_self_hosted_lexer_scaffold_skips_comments_and_normalizes_number_lexemes() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = common::unique_fixture_root("run-self-hosted-lexer-comments-numbers");
    let source_path = fixture_root.join("numbers.tn");
    fs::write(&source_path, "value_1 = 1_000 #x\nnext <= 3.5\n")
        .expect("fixture setup should write comment/number source");

    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&repo_root)
        .args([
            "run",
            "examples/apps/self_hosted_lexer",
            source_path.to_str().expect("source path should be utf8"),
        ])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected self_hosted_lexer run success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    for needle in [
        ":source_length => 31",
        ":tokens => [%{:kind => \"IDENT\", :lexeme => \"value_1\", :span_start => 0, :span_end => 7}, %{:kind => \"MATCH_EQ\", :lexeme => \"\", :span_start => 8, :span_end => 9}, %{:kind => \"INT\", :lexeme => \"1000\", :span_start => 10, :span_end => 15}, %{:kind => \"IDENT\", :lexeme => \"next\", :span_start => 19, :span_end => 23}, %{:kind => \"LT_EQ\", :lexeme => \"\", :span_start => 24, :span_end => 26}, %{:kind => \"FLOAT\", :lexeme => \"3.5\", :span_start => 27, :span_end => 30}, %{:kind => \"EOF\", :lexeme => \"\", :span_start => 31, :span_end => 31}]",
    ] {
        assert!(
            stdout.contains(needle),
            "expected self-hosted lexer output to contain '{needle}', got: {stdout}"
        );
    }
}
