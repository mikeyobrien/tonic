use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use serde_json::Value;
use std::fs;
mod common;

#[test]
fn compile_llvm_emit_executable_writes_manifest_and_native_artifacts() {
    let temp_dir = common::unique_temp_dir("compile-llvm-executable");
    let source_path = temp_dir.join("native.tn");
    fs::write(
        &source_path,
        "defmodule Native do\n  def run() do\n    1 + 2\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args([
            "compile",
            "native.tn",
            "--backend",
            "llvm",
            "--emit",
            "executable",
        ])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected llvm executable compile to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("compile: ok"));
    assert!(stdout.contains(".tonic/build/native.tnx.json"));

    let manifest_path = temp_dir.join(".tonic/build/native.tnx.json");
    let manifest_raw = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let manifest: Value = serde_json::from_str(&manifest_raw).expect("manifest should be json");

    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["backend"], "llvm");
    assert_eq!(manifest["emit"], "executable");
    assert_eq!(
        manifest["target_triple"],
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    );
    assert_eq!(manifest["tonic_version"], env!("CARGO_PKG_VERSION"));

    let ir_path = temp_dir.join(".tonic/build/native.tir.json");
    let ll_path = temp_dir.join(".tonic/build/native.ll");
    let object_path = temp_dir.join(".tonic/build/native.o");

    assert!(
        ir_path.exists(),
        "expected IR artifact at {}",
        ir_path.display()
    );
    assert!(
        ll_path.exists(),
        "expected LLVM IR artifact at {}",
        ll_path.display()
    );
    assert!(
        object_path.exists(),
        "expected object artifact at {}",
        object_path.display()
    );
}

#[test]
fn run_executes_native_artifact_manifest_with_interpreter_compatible_output() {
    let temp_dir = common::unique_temp_dir("run-native-artifact-manifest");
    let source_path = temp_dir.join("native_run.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    40 + 2\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args([
            "compile",
            "native_run.tn",
            "--backend",
            "llvm",
            "--emit",
            "executable",
        ])
        .assert()
        .success();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", ".tonic/build/native_run.tnx.json"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("42\n"));
}

#[test]
fn compile_rejects_unknown_emit_mode() {
    let temp_dir = common::unique_temp_dir("compile-unknown-emit");
    let source_path = temp_dir.join("emit.tn");
    fs::write(
        &source_path,
        "defmodule Emit do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "emit.tn", "--emit", "nope"])
        .assert()
        .failure()
        .stderr(contains("error: unsupported emit mode 'nope'"));
}

#[test]
fn run_native_artifact_rejects_target_mismatch_with_deterministic_diagnostic() {
    let temp_dir = common::unique_temp_dir("run-native-artifact-target-mismatch");
    let source_path = temp_dir.join("native_target.tn");
    fs::write(
        &source_path,
        "defmodule NativeTarget do\n  def run() do\n    7\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args([
            "compile",
            "native_target.tn",
            "--backend",
            "llvm",
            "--emit",
            "executable",
        ])
        .assert()
        .success();

    let manifest_path = temp_dir.join(".tonic/build/native_target.tnx.json");
    let manifest_raw = fs::read_to_string(&manifest_path).expect("manifest should be readable");
    let mut manifest: Value = serde_json::from_str(&manifest_raw).expect("manifest should be json");
    manifest["target_triple"] = Value::String("bogus-target".to_string());
    fs::write(
        &manifest_path,
        serde_json::to_string(&manifest).expect("manifest should serialize"),
    )
    .expect("manifest rewrite should succeed");

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", ".tonic/build/native_target.tnx.json"])
        .assert()
        .failure()
        .stderr(contains(
            "error: native artifact target mismatch: artifact=bogus-target",
        ));
}
