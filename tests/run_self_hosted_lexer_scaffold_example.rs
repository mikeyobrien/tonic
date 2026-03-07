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
fn run_self_hosted_lexer_scaffold_reads_source_path_and_emits_placeholder_payload() {
    let repo_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_root = common::unique_fixture_root("run-self-hosted-lexer-scaffold");
    let source_path = fixture_root.join("sample.tn");
    fs::write(&source_path, "a + b\n").expect("fixture setup should write sample source");

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
        "expected self_hosted_lexer scaffold run success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    for needle in [
        "%{:source_path => \"",
        source_path.to_str().expect("source path should be utf8"),
        ":source_length => 6",
        ":tokens => [%{:kind => :stub, :lexeme => \"\", :span_start => 0, :span_end => 0}]",
    ] {
        assert!(
            stdout.contains(needle),
            "expected scaffold output to contain '{needle}', got: {stdout}"
        );
    }
}
