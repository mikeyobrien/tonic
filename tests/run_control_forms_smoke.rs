use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_if_unless_cond_and_with_happy_path() {
    let fixture_root = unique_fixture_root("run-control-forms-happy");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_control_forms_happy.tn"),
        "defmodule Demo do\n  def classify(value) do\n    cond do\n      value > 10 -> 100\n      value > 5 -> 50\n      true -> 5\n    end\n  end\n\n  def run() do\n    with [left, right] <- list(6, 7),\n         extra <- if left > 5 do\n           1\n         else\n           0\n         end do\n      unless false do\n        left + right + extra + classify(left)\n      else\n        0\n      end\n    else\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write control forms source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_control_forms_happy.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "64\n");
}

#[test]
fn run_executes_with_else_fallback_on_pattern_mismatch() {
    let fixture_root = unique_fixture_root("run-control-forms-with-else");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_control_forms_with_else.tn"),
        "defmodule Demo do\n  def run() do\n    with [left, right] <- list(9),\n         value <- left + right do\n      value\n    else\n      [single] -> single + 40\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write with fallback source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_control_forms_with_else.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "49\n");
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
