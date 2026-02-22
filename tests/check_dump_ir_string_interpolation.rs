use std::fs;
mod common;

#[test]
fn check_dump_ir_supports_string_interpolation() {
    let fixture_root = common::unique_fixture_root("check-dump-ir-string-interpolation");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("interpolation.tn"),
        "defmodule Demo do\n  def run() do\n    \"hello #{1 + 2} world\"\n  end\nend\n",
    )
    .expect("fixture setup should write interpolation source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/interpolation.tn", "--dump-ir"])
        .output()
        .expect("check command should execute");

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("const_string"));
    assert!(stdout.contains("to_string"));
    assert!(stdout.contains("concat"));
}
