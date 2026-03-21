use std::fs;
mod common;

#[test]
fn check_reports_missing_map_pattern_fat_arrow_parse_error() {
    let fixture_root = common::unique_fixture_root("check-map-pattern-fat-arrow-diagnostics");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("map_pattern_fat_arrow_invalid.tn"),
        "defmodule Demo do\n  def run(value) do\n    case value do\n      %{1 payload} -> payload\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write invalid map pattern source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/map_pattern_fat_arrow_invalid.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for malformed map pattern syntax"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("[E0008] missing '=>' in map pattern entry; found IDENT(payload) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: write `%{key => pattern}` for computed keys"),
        "unexpected parser diagnostic: {stderr}"
    );
}
