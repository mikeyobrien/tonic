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
fn run_executes_for_with_generator_guards() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-generator-guard");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_generator_guard.tn"),
        "defmodule Demo do\n  def run() do\n    for x when x > 2 <- list(1, 2, 3, 4) do\n      x\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for generator guard source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_generator_guard.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[3, 4]\n");
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
fn run_executes_for_reduce_accumulator_mode() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-reduce");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_reduce.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2, 3), reduce: 0 do\n      acc -> acc + x\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for reduce source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_reduce.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "6\n");
}

#[test]
fn run_executes_for_into_map_destination() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-into-map");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_into_map.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), into: map(:seed, 0) do\n      tuple(x, x * 10)\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for map destination source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_into_map.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "%{:seed => 0, 1 => 10, 2 => 20}\n");
}

#[test]
fn run_executes_for_into_keyword_destination() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-into-keyword");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_into_keyword.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), into: [seed: 0] do\n      tuple(:v, x)\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write for keyword destination source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_into_keyword.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[seed: 0, v: 1, v: 2]\n");
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

#[test]
fn run_reports_deterministic_error_for_invalid_map_into_yield() {
    let fixture_root = common::unique_fixture_root("run-comprehensions-map-into-invalid-yield");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("for_invalid_map_into.tn"),
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), into: %{} do\n      x\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write invalid map into source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/for_invalid_map_into.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        !output.status.success(),
        "expected invalid map-into yield run failure, got status {:?} and stdout: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout)
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("for into map expects tuple {key, value}, found int"),
        "expected deterministic map-into yield diagnostic, stderr was: {stderr}"
    );
}
