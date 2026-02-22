use std::fs;
mod common;

#[test]
fn run_executes_logical_operators_and_prints_rendered_value() {
    let fixture_root = common::unique_fixture_root("run-logical");
    let project_dir = fixture_root.join("logical_project");

    fs::create_dir_all(&project_dir).expect("fixture setup should create project directory");

    fs::write(
        project_dir.join("tonic.toml"),
        "[project]\nname = \"logical_project\"\nentry = \"main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        project_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    not false and (true or false) && 1 || 2\n  end\nend\n",
    )
    .expect("fixture setup should write main.tn");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_dir)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "1");
}

#[test]
fn run_executes_collection_operators_and_prints_rendered_value() {
    let fixture_root = common::unique_fixture_root("run-collection");
    let project_dir = fixture_root.join("collection_project");

    fs::create_dir_all(&project_dir).expect("fixture setup should create project directory");

    fs::write(
        project_dir.join("tonic.toml"),
        "[project]\nname = \"collection_project\"\nentry = \"main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        project_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    \"hello\" <> \" \" <> \"world\"\n  end\nend\n",
    )
    .expect("fixture setup should write main.tn");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_dir)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "\"hello world\"");
}

#[test]
fn run_short_circuit_operators_do_not_eagerly_evaluate_rhs() {
    let fixture_root = common::unique_fixture_root("run-short-circuit");
    let project_dir = fixture_root.join("short_circuit_project");

    fs::create_dir_all(&project_dir).expect("fixture setup should create project directory");

    fs::write(
        project_dir.join("tonic.toml"),
        "[project]\nname = \"short_circuit_project\"\nentry = \"main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        project_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(tuple(false && (1 / 0), true || (1 / 0)), tuple(false and (1 / 0), true or (1 / 0)))\n  end\nend\n",
    )
    .expect("fixture setup should write main.tn");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_dir)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "{{false, true}, {false, true}}");
}

#[test]
fn run_executes_relaxed_bang_and_membership_range_operators() {
    let fixture_root = common::unique_fixture_root("run-bang-in-range");
    let project_dir = fixture_root.join("bang_in_range_project");

    fs::create_dir_all(&project_dir).expect("fixture setup should create project directory");

    fs::write(
        project_dir.join("tonic.toml"),
        "[project]\nname = \"bang_in_range_project\"\nentry = \"main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        project_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(tuple(!nil, !1), tuple(2 in 1..3, 4 in 1..3))\n  end\nend\n",
    )
    .expect("fixture setup should write main.tn");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_dir)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "{{true, false}, {true, false}}");
}
