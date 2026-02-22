use std::fs;
mod common;

#[test]
fn check_dump_ast_matches_single_module_two_function_contract() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-module");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("parser_smoke.tn"),
        "defmodule Math do\n  def one() do\n    1\n  end\n\n  def two() do\n    one()\n  end\nend\n",
    )
    .expect("fixture setup should write parser smoke source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/parser_smoke.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"Math\",\"functions\":[",
        "{\"name\":\"one\",\"params\":[],\"body\":{\"kind\":\"int\",\"value\":1}},",
        "{\"name\":\"two\",\"params\":[],\"body\":{\"kind\":\"call\",\"callee\":\"one\",\"args\":[]}}",
        "]}]}\n"
    );

    assert_eq!(stdout, expected);
}
