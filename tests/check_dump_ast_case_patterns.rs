use std::fs;
mod common;

#[test]
fn check_dump_ast_matches_case_pattern_contract() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-case-patterns");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("parser_case_patterns.tn"),
        "defmodule PatternDemo do\n  def run() do\n    case input() do\n      {:ok, value} -> 1\n      [head, tail] -> 2\n      %{} -> 3\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write parser case-pattern source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/parser_case_patterns.tn", "--dump-ast"])
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
        "{\"modules\":[{\"name\":\"PatternDemo\",\"functions\":[",
        "{\"name\":\"run\",\"params\":[],\"body\":{",
        "\"kind\":\"case\",",
        "\"subject\":{\"kind\":\"call\",\"callee\":\"input\",\"args\":[]},",
        "\"branches\":[",
        "{\"pattern\":{\"kind\":\"tuple\",\"items\":[",
        "{\"kind\":\"atom\",\"value\":\"ok\"},",
        "{\"kind\":\"bind\",\"name\":\"value\"}",
        "]},\"body\":{\"kind\":\"int\",\"value\":1}},",
        "{\"pattern\":{\"kind\":\"list\",\"items\":[",
        "{\"kind\":\"bind\",\"name\":\"head\"},",
        "{\"kind\":\"bind\",\"name\":\"tail\"}",
        "]},\"body\":{\"kind\":\"int\",\"value\":2}},",
        "{\"pattern\":{\"kind\":\"map\",\"entries\":[]},\"body\":{\"kind\":\"int\",\"value\":3}}",
        "]",
        "}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}

#[test]
fn check_dump_ast_supports_literal_case_pattern_variants() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-case-pattern-literals");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("parser_case_pattern_literals.tn"),
        "defmodule PatternDemo do\n  def run() do\n    case input() do\n      true -> 1\n      nil -> 2\n      \"ok\" -> 3\n      _ -> 4\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write parser case-pattern literal source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args([
            "check",
            "examples/parser_case_pattern_literals.tn",
            "--dump-ast",
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
    let expected = concat!(
        "{\"modules\":[{\"name\":\"PatternDemo\",\"functions\":[",
        "{\"name\":\"run\",\"params\":[],\"body\":{",
        "\"kind\":\"case\",",
        "\"subject\":{\"kind\":\"call\",\"callee\":\"input\",\"args\":[]},",
        "\"branches\":[",
        "{\"pattern\":{\"kind\":\"bool\",\"value\":true},\"body\":{\"kind\":\"int\",\"value\":1}},",
        "{\"pattern\":{\"kind\":\"nil\"},\"body\":{\"kind\":\"int\",\"value\":2}},",
        "{\"pattern\":{\"kind\":\"string\",\"value\":\"ok\"},\"body\":{\"kind\":\"int\",\"value\":3}},",
        "{\"pattern\":{\"kind\":\"wildcard\"},\"body\":{\"kind\":\"int\",\"value\":4}}",
        "]",
        "}}]}",
        "]}\n"
    );

    assert_eq!(stdout, expected);
}
