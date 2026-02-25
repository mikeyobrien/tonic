use std::fs;
mod common;

#[test]
fn run_executes_map_literal_with_fat_arrow_keys() {
    let fixture_root = common::unique_fixture_root("run-map-fat-arrow-literal");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("map_fat_arrow_literal.tn"),
        "defmodule Demo do\n  def run() do\n    %{\"status\" => 200, 1 => true, false => :nope}\n  end\nend\n",
    )
    .expect("fixture setup should write map literal source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/map_fat_arrow_literal.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "%{\"status\" => 200, 1 => true, false => :nope}\n");
}

#[test]
fn run_executes_case_map_pattern_with_fat_arrow_keys() {
    let fixture_root = common::unique_fixture_root("run-map-fat-arrow-pattern");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("map_fat_arrow_pattern.tn"),
        "defmodule Demo do\n  def run() do\n    case %{\"status\" => 200, 1 => true} do\n      %{\"status\" => code, 1 => ok} -> if ok do\n        code\n      else\n        0\n      end\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write map-pattern source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/map_fat_arrow_pattern.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "200\n");
}
