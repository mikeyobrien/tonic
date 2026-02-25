use std::fs;
mod common;

#[test]
fn check_dump_ast_includes_module_forms_and_attributes() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-module-forms");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("module_forms_ast.tn"),
        "defmodule Math do\n  def helper() do\n    7\n  end\nend\n\ndefmodule Demo do\n  alias Math, as: M\n  import Math\n  require Logger\n  use Feature\n  @moduledoc \"demo module\"\n  @doc \"run docs\"\n  @answer 5\n\n  def run() do\n    M.helper() + helper()\n  end\nend\n",
    )
    .expect("fixture setup should write module forms source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/module_forms_ast.tn", "--dump-ast"])
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
    assert_eq!(
        ast["modules"][1]["forms"],
        serde_json::json!([
            {"kind":"alias","module":"Math","as":"M"},
            {"kind":"import","module":"Math"},
            {"kind":"require","module":"Logger"},
            {"kind":"use","module":"Feature"}
        ])
    );
    assert_eq!(
        ast["modules"][1]["attributes"],
        serde_json::json!([
            {"name":"moduledoc","value":{"kind":"string","value":"demo module"}},
            {"name":"doc","value":{"kind":"string","value":"run docs"}},
            {"name":"answer","value":{"kind":"int","value":5}}
        ])
    );
    assert_eq!(
        ast["modules"][1]["functions"][0]["body"],
        serde_json::json!({
            "kind":"binary",
            "op":"plus",
            "left":{"kind":"call","callee":"Math.helper","args":[]},
            "right":{"kind":"call","callee":"Math.helper","args":[]}
        })
    );
}

#[test]
fn check_dump_ast_includes_import_only_and_except_filters() {
    let fixture_root = common::unique_fixture_root("check-dump-ast-import-filters");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("module_forms_import_filters.tn"),
        "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\n\n  def unsafe(value) do\n    value - 1\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [add: 2]\n  import Math, except: [unsafe: 1]\n\n  def run() do\n    add(20, 22)\n  end\nend\n",
    )
    .expect("fixture setup should write module forms source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args([
            "check",
            "examples/module_forms_import_filters.tn",
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
    let ast: serde_json::Value = serde_json::from_str(&stdout).expect("stdout should be json");

    assert_eq!(
        ast["modules"][1]["forms"],
        serde_json::json!([
            {"kind":"import","module":"Math","only":[{"name":"add","arity":2}]},
            {"kind":"import","module":"Math","except":[{"name":"unsafe","arity":1}]}
        ])
    );
    assert_eq!(
        ast["modules"][1]["functions"][0]["body"],
        serde_json::json!({
            "kind":"call",
            "callee":"Math.add",
            "args":[
                {"kind":"int","value":20},
                {"kind":"int","value":22}
            ]
        })
    );
}
