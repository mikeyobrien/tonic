use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_help_lists_usage() {
    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));

    cmd.arg("compile")
        .arg("--help")
        .assert()
        .success()
        .stdout(contains(
            "tonic compile <path> [--backend <interp|llvm>] [--emit <ir|llvm-ir|object|executable>] [--out <artifact-path>|--dump-mir]",
        ));
}

#[test]
fn compile_single_file_success() {
    let temp_dir = common::unique_temp_dir("single-file");
    let source_path = temp_dir.join("single.tn");
    fs::write(
        &source_path,
        "defmodule Single do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg("single.tn")
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"))
        .stdout(contains(".tonic/build/single.tir.json"));

    let artifact_path = temp_dir.join(".tonic/build/single.tir.json");
    assert!(artifact_path.exists());
    let content = fs::read_to_string(&artifact_path).unwrap();
    assert!(content.contains(r#""Single.run""#));
}

#[test]
fn compile_project_root_success() {
    let temp_dir = common::unique_temp_dir("project-root");
    let src_dir = temp_dir.join("src");
    fs::create_dir_all(&src_dir).unwrap();

    fs::write(
        temp_dir.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .unwrap();
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg(".")
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"))
        .stdout(contains(".tonic/build/main.tir.json"));

    let artifact_path = temp_dir.join(".tonic/build/main.tir.json");
    assert!(artifact_path.exists());
    let content = fs::read_to_string(&artifact_path).unwrap();
    assert!(content.contains(r#""Demo.run""#));
}

#[test]
fn compile_custom_out_path() {
    let temp_dir = common::unique_temp_dir("custom-out");
    let source_path = temp_dir.join("custom.tn");
    fs::write(
        &source_path,
        "defmodule Custom do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let custom_out_path = temp_dir.join("out-dir").join("my-artifact.json");

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg("custom.tn")
        .arg("--out")
        .arg(custom_out_path.to_str().unwrap())
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"))
        .stdout(contains("my-artifact.json"));

    assert!(custom_out_path.exists());
    let content = fs::read_to_string(&custom_out_path).unwrap();
    assert!(content.contains(r#""Custom.run""#));
}

#[test]
fn compile_artifact_content_is_deterministic() {
    let temp_dir = common::unique_temp_dir("deterministic");
    let source_path = temp_dir.join("deterministic.tn");
    fs::write(
        &source_path,
        "defmodule Deterministic do\n  def run() do\n    1 + 2\n  end\nend\n",
    )
    .unwrap();

    let out_a = temp_dir.join("out-a.json");
    let out_b = temp_dir.join("out-b.json");

    let mut first = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    first.current_dir(&temp_dir);
    first
        .arg("compile")
        .arg("deterministic.tn")
        .arg("--out")
        .arg(out_a.to_str().unwrap())
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let mut second = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    second.current_dir(&temp_dir);
    second
        .arg("compile")
        .arg("deterministic.tn")
        .arg("--out")
        .arg(out_b.to_str().unwrap())
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let first_artifact = fs::read_to_string(out_a).unwrap();
    let second_artifact = fs::read_to_string(out_b).unwrap();

    assert_eq!(first_artifact, second_artifact);
}

#[test]
fn compile_failure_invalid_source() {
    let temp_dir = common::unique_temp_dir("invalid-source");
    let source_path = temp_dir.join("invalid.tn");
    fs::write(
        &source_path,
        "defmodule Invalid \n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg("invalid.tn")
        .assert()
        .failure()
        .stderr(contains("error:"));
}
