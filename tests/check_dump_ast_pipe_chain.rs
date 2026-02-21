use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_ast_matches_pipe_chain_contract() {
    let fixture_root = unique_fixture_root("check-dump-ast-pipe-chain");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("parser_pipe_chain.tn"),
        "defmodule Pipes do\n  def run() do\n    source() |> normalize() |> persist(1)\n  end\nend\n",
    )
    .expect("fixture setup should write parser pipe-chain source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/parser_pipe_chain.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let expected = concat!(
        "{\"modules\":[{\"name\":\"Pipes\",\"functions\":[",
        "{\"name\":\"run\",\"params\":[],\"body\":{",
        "\"kind\":\"pipe\",",
        "\"left\":{",
        "\"kind\":\"pipe\",",
        "\"left\":{\"kind\":\"call\",\"callee\":\"source\",\"args\":[]},",
        "\"right\":{\"kind\":\"call\",\"callee\":\"normalize\",\"args\":[]}",
        "},",
        "\"right\":{\"kind\":\"call\",\"callee\":\"persist\",\"args\":[{\"kind\":\"int\",\"value\":1}]}",
        "}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
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
