use std::fs;
mod common;

#[test]
fn run_executes_for_single_generator_comprehension() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-single-generator");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_single_generator.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2, 3) do\n      x + 1\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for comprehension source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_single_generator.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[2, 3, 4]\n");
}

#[test]
fn run_executes_for_with_pattern_filtering() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-pattern-filter");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_pattern_filter.tn"),
        "defmodule Demo do\n  def run() do\n    for [left, right] <- list(list(1, 2), 9, list(4, 5)) do\n      left + right\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for pattern filter source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_pattern_filter.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[3, 9]\n");
}

#[test]
fn run_executes_for_multi_generator_comprehension() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-multi-generator");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_multi_generator.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), y <- list(3, 4) do\n      list(x, y)\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for multi generator source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_multi_generator.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[[1, 3], [1, 4], [2, 3], [2, 4]]\n");
}

#[test]
fn run_reports_deterministic_error_for_non_list_generator() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-non-list-generator");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_non_list_generator.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- 1 do\n      x\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write non-list generator source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_non_list_generator.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected non-list generator run failure, got status {:?} and stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("for expects list generator, found int"),
        "expected deterministic non-list generator runtime diagnostic, stderr was: {stderr}"
    );
}
