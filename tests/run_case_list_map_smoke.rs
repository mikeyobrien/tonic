use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_case_with_list_pattern() {
    let fixture_root = unique_fixture_root("run-case-list-pattern");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_case_list_pattern.tn"),
        "defmodule Demo do\n  def run() do\n    case list(1, 2) do\n      [head, tail] -> head + tail\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write list-pattern source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_case_list_pattern.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "3\n");
}

#[test]
fn run_executes_case_with_nested_map_and_list_patterns() {
    let fixture_root = unique_fixture_root("run-case-map-list-pattern");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_case_map_pattern.tn"),
        "defmodule Demo do\n  def run() do\n    case map(:ok, list(7, 8)) do\n      %{:ok -> [value, _]} -> value + 0\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write map-pattern source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_case_map_pattern.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "7\n");
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
