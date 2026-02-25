use std::fs;
mod common;

#[test]
fn check_dump_ast_includes_struct_forms_literals_updates_and_patterns() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-struct-syntax");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("struct_syntax_ast.tn"),
        "defmodule User do\n  defstruct name: \"\", age: 0\n\n  def run(user) do\n    case %User{user | age: 43} do\n      %User{name: name} -> %User{name: name}\n      _ -> %User{}\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write struct syntax source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/struct_syntax_ast.tn", "--dump-ast"])
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

    assert_eq!(
        ast["modules"][0]["forms"],
        serde_json::json!([
            {
                "kind":"defstruct",
                "fields":[
                    {"name":"name","default":{"kind":"string","value":""}},
                    {"name":"age","default":{"kind":"int","value":0}}
                ]
            }
        ])
    );

    assert_eq!(
        ast["modules"][0]["functions"][0]["body"]["kind"],
        serde_json::json!("case")
    );
    assert_eq!(
        ast["modules"][0]["functions"][0]["body"]["subject"]["kind"],
        serde_json::json!("structupdate")
    );
    assert_eq!(
        ast["modules"][0]["functions"][0]["body"]["branches"][0]["pattern"]["kind"],
        serde_json::json!("struct")
    );
}
