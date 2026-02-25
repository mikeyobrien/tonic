use std::fs;
mod common;

#[test]
fn run_executes_struct_literal_update_and_pattern_match() {
    let fixture_root = common::unique_fixture_root("run-struct-syntax-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_struct_syntax.tn"),
        "defmodule User do\n  defstruct name: \"\", age: 0\nend\n\ndefmodule Demo do\n  def run() do\n    case %User{%User{name: \"A\", age: 42} | age: 43} do\n      %User{name: name, age: age} -> {%User{name: name, age: age}, {name, age}}\n      _ -> :error\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write struct syntax source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_struct_syntax.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected run command to succeed, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout,
        "{%{:__struct__ => :User, :name => \"A\", :age => 43}, {\"A\", 43}}\n"
    );
}
