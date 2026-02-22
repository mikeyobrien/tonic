use std::fs;
use std::path::PathBuf;

#[test]
fn run_dispatches_function_clauses_by_pattern_order() {
    let fixture_root = unique_fixture_root("run-function-clauses-pattern");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_function_clauses_pattern.tn"),
        "defmodule Demo do\n  def classify({:ok, value}) do\n    value\n  end\n\n  def classify(_) do\n    0\n  end\n\n  def run() do\n    classify({:ok, 9})\n  end\nend\n",
    )
    .expect("fixture setup should write function clause source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_function_clauses_pattern.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "9\n");
}

#[test]
fn run_supports_function_default_arguments() {
    let fixture_root = unique_fixture_root("run-function-defaults");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_function_defaults.tn"),
        "defmodule Demo do\n  def add(value, increment \\\\ 2) do\n    value + increment\n  end\n\n  def run() do\n    add(5)\n  end\nend\n",
    )
    .expect("fixture setup should write default argument source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_function_defaults.tn"])
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

#[test]
fn check_rejects_cross_module_calls_to_private_functions() {
    let fixture_root = unique_fixture_root("check-defp-visibility");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("check_defp_visibility.tn"),
        "defmodule Math do\n  defp hidden() do\n    7\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.hidden()\n  end\nend\n",
    )
    .expect("fixture setup should write defp visibility source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", "examples/check_defp_visibility.tn"])
        .output()
        .expect("check command should execute");

    assert_eq!(output.status.code(), Some(1));

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert_eq!(
        stderr,
        "error: [E1002] private function 'Math.hidden' cannot be called from Demo.run\n"
    );
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
