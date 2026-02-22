use std::fs;
use std::path::PathBuf;

#[test]
fn check_dump_ast_lowers_if_unless_cond_and_with_to_case_contracts() {
    let fixture_root = unique_fixture_root("check-dump-ast-control-forms");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("control_forms.tn"),
        "defmodule Demo do\n  def pick(flag) do\n    if flag do\n      1\n    else\n      2\n    end\n  end\n\n  def reject(flag) do\n    unless flag do\n      3\n    else\n      4\n    end\n  end\n\n  def route(value) do\n    cond do\n      value > 10 -> 1\n      value > 5 -> 2\n      true -> 3\n    end\n  end\n\n  def chain() do\n    with [left, right] <- list(1, 2),\n         total <- left + right do\n      total\n    else\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write control forms source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/control_forms.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid json");
    let functions = &json["modules"][0]["functions"];

    assert_eq!(functions[0]["name"], "pick");
    assert_eq!(functions[0]["body"]["kind"], "case");
    assert_eq!(
        functions[0]["body"]["branches"][0]["pattern"]["kind"],
        "wildcard"
    );
    assert_eq!(
        functions[0]["body"]["branches"][0]["guard"]["kind"],
        "unary"
    );

    assert_eq!(functions[1]["name"], "reject");
    assert_eq!(functions[1]["body"]["kind"], "case");
    assert_eq!(
        functions[1]["body"]["branches"][0]["guard"]["kind"],
        "unary"
    );

    assert_eq!(functions[2]["name"], "route");
    assert_eq!(functions[2]["body"]["kind"], "case");
    assert_eq!(
        functions[2]["body"]["branches"].as_array().unwrap().len(),
        3
    );
    assert_eq!(
        functions[2]["body"]["branches"][2]["guard"]["kind"],
        "unary"
    );

    assert_eq!(functions[3]["name"], "chain");
    assert_eq!(functions[3]["body"]["kind"], "case");
    assert_eq!(
        functions[3]["body"]["branches"][0]["pattern"]["kind"],
        "list"
    );
    assert_eq!(functions[3]["body"]["branches"][1]["body"]["kind"], "case");
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
