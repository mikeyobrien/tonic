use std::fs;
mod common;

#[test]
fn check_dump_ast_includes_defprotocol_and_defimpl_forms() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-protocol-forms");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("protocol_forms_ast.tn"),
        "defmodule User do\n  defstruct age: 0\nend\n\ndefmodule Demo do\n  defprotocol Size do\n    def size(value)\n  end\n\n  defimpl Size, for: Tuple do\n    def size(_value) do\n      2\n    end\n  end\n\n  defimpl Size, for: User do\n    def size(user) do\n      user.age\n    end\n  end\n\n  def run(user) do\n    tuple(Size.size(tuple(1, 2)), Size.size(user))\n  end\nend\n",
    )
    .expect("fixture setup should write protocol forms source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/protocol_forms_ast.tn", "--dump-ast"])
        .output()
        .expect("check command should run");

    assert!(
        output.status.success(),
        "expected successful check invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let ast: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");

    assert_eq!(ast["modules"][1]["name"], "Demo");
    assert_eq!(ast["modules"][1]["forms"][0]["kind"], "defprotocol");
    assert_eq!(ast["modules"][1]["forms"][0]["name"], "Size");
    assert_eq!(
        ast["modules"][1]["forms"][0]["functions"],
        serde_json::json!([
            {"name": "size", "params": ["value"]}
        ])
    );

    assert_eq!(ast["modules"][1]["forms"][1]["kind"], "defimpl");
    assert_eq!(ast["modules"][1]["forms"][1]["protocol"], "Size");
    assert_eq!(ast["modules"][1]["forms"][1]["for"], "Tuple");
    assert_eq!(ast["modules"][1]["forms"][2]["kind"], "defimpl");
    assert_eq!(ast["modules"][1]["forms"][2]["for"], "User");
}
