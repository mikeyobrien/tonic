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
        .stdout(contains("tonic compile <path> [--out <artifact-path>]"));
}

#[test]
fn compile_emit_flag_fails_with_usage_error() {
    let temp_dir = common::unique_temp_dir("compile-emit-rejected");
    let source_path = temp_dir.join("hello.tn");
    fs::write(
        &source_path,
        "defmodule Hello do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg("hello.tn")
        .arg("--emit")
        .arg("executable")
        .assert()
        .failure()
        .stderr(contains("error: unexpected argument '--emit'"));
}

#[test]
fn compile_emit_any_value_fails_with_usage_error() {
    let temp_dir = common::unique_temp_dir("compile-emit-any-value");
    let source_path = temp_dir.join("hello.tn");
    fs::write(
        &source_path,
        "defmodule Hello do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    for emit_value in &["ir", "llvm-ir", "object", "executable"] {
        let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
        cmd.current_dir(&temp_dir);
        cmd.arg("compile")
            .arg("hello.tn")
            .arg("--emit")
            .arg(emit_value)
            .assert()
            .failure()
            .stderr(contains("error: unexpected argument '--emit'"));
    }
}

#[test]
fn compile_backend_flag_is_not_supported() {
    let temp_dir = common::unique_temp_dir("compile-backend-unsupported");
    let source_path = temp_dir.join("hello.tn");
    fs::write(
        &source_path,
        "defmodule Hello do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg("hello.tn")
        .arg("--backend")
        .arg("llvm")
        .assert()
        .failure()
        .stderr(contains("error: unexpected argument '--backend'"));
}

#[test]
fn compile_dump_mir_flag_is_not_supported() {
    let temp_dir = common::unique_temp_dir("compile-dump-mir-unsupported");
    let source_path = temp_dir.join("hello.tn");
    fs::write(
        &source_path,
        "defmodule Hello do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg("hello.tn")
        .arg("--dump-mir")
        .assert()
        .failure()
        .stderr(contains("error: unexpected argument '--dump-mir'"));
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
        .stdout(contains(".tonic/build/single"));

    let artifact_path = temp_dir.join(".tonic/build/single");
    assert!(artifact_path.exists());
    let bytes = fs::read(&artifact_path).unwrap();
    assert_eq!(&bytes[..4], b"\x7fELF");
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
        .stdout(contains(".tonic/build/main"));

    let artifact_path = temp_dir.join(".tonic/build/main");
    assert!(artifact_path.exists());
    let bytes = fs::read(&artifact_path).unwrap();
    assert_eq!(&bytes[..4], b"\x7fELF");
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

    let custom_out_path = temp_dir.join("out-dir").join("my-artifact");

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
        .stdout(contains("my-artifact"));

    assert!(custom_out_path.exists());
    let bytes = fs::read(&custom_out_path).unwrap();
    assert_eq!(&bytes[..4], b"\x7fELF");
}

#[test]
fn compile_out_path_is_directory_is_usage_error() {
    let temp_dir = common::unique_temp_dir("compile-out-is-dir");
    let source_path = temp_dir.join("hello.tn");
    fs::write(
        &source_path,
        "defmodule Hello do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    // Create a directory at the --out path so the guard triggers.
    let out_dir = temp_dir.join("existing-dir");
    fs::create_dir_all(&out_dir).unwrap();

    let mut cmd = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"));
    cmd.current_dir(&temp_dir);

    cmd.arg("compile")
        .arg("hello.tn")
        .arg("--out")
        .arg(out_dir.to_str().unwrap())
        .assert()
        .failure()
        .code(64)
        .stderr(contains("error: --out path"))
        .stderr(contains("is a directory"));
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
