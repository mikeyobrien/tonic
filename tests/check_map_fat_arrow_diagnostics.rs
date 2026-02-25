use std::fs;
mod common;

#[test]
fn check_reports_deterministic_map_fat_arrow_parse_error() {
    let fixture_root = common::unique_fixture_root("check-map-fat-arrow-diagnostics");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("map_fat_arrow_invalid.tn"),
        "defmodule Demo do\n  def run() do\n    %{1 2}\n  end\nend\n",
    )
    .expect("fixture setup should write invalid map source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/map_fat_arrow_invalid.tn"])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for malformed map syntax"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("expected map fat arrow `=>`, found INT(2)"),
        "unexpected parser diagnostic: {stderr}"
    );
}
