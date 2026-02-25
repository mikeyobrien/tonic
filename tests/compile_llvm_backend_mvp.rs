use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_llvm_backend_emits_executable_artifact_for_subset_program() {
    let temp_dir = common::unique_temp_dir("compile-llvm-subset");
    let source_path = temp_dir.join("math.tn");
    fs::write(
        &source_path,
        "defmodule Math do\n  def add(a, b) do\n    a + b\n  end\n\n  def run() do\n    add(7, 8) > 10\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "math.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected llvm compile to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("compile: ok"));
    // compile: ok must report the executable (no extension), not the manifest
    assert!(
        stdout.contains(".tonic/build/math"),
        "expected executable path in stdout: {stdout}"
    );
    assert!(
        !stdout.trim_end().ends_with(".tnx.json"),
        "compile: ok must point to executable, not manifest: {stdout}"
    );

    // Real ELF binary at the reported path
    let exe_path = temp_dir.join(".tonic/build/math");
    assert!(exe_path.exists(), "ELF executable should exist");

    let elf_bytes = fs::read(&exe_path).expect("should read ELF binary");
    assert_eq!(
        &elf_bytes[..4],
        b"\x7fELF",
        "output must be a real ELF binary"
    );

    // LLVM IR sidecar is still generated
    let ll_path = temp_dir.join(".tonic/build/math.ll");
    assert!(
        ll_path.exists(),
        "expected .ll sidecar at {}",
        ll_path.display()
    );

    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir sidecar should be readable");
    assert!(llvm_ir.contains("; tonic llvm backend mvp"));
    assert!(llvm_ir.contains("define i64 @tn_Math_add__arity2(i64 %arg0, i64 %arg1)"));
    assert!(llvm_ir.contains("define i64 @tn_Math_run__arity0()"));
    assert!(llvm_ir.contains("call i64 @tn_Math_add__arity2"));
    assert!(llvm_ir.contains("icmp sgt i64"));
    assert!(llvm_ir.contains("define i64 @main()"));
}

#[test]
fn compile_llvm_backend_keeps_for_reduce_option_failure_deterministic() {
    let temp_dir = common::unique_temp_dir("compile-llvm-unsupported");
    let source_path = temp_dir.join("unsupported.tn");
    fs::write(
        &source_path,
        "defmodule Unsupported do\n  def run() do\n    for x <- [1, 2, 3], reduce: 0 do\n      x\n    end\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "unsupported.tn"])
        .assert()
        .failure()
        .stderr(contains(
            "error: unsupported for option 'reduce'; remove options from for for now",
        ));
}

#[test]
fn compile_rejects_backend_flag() {
    let temp_dir = common::unique_temp_dir("compile-backend-unexpected");
    let source_path = temp_dir.join("single.tn");
    fs::write(
        &source_path,
        "defmodule Single do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "single.tn", "--backend", "llvm"])
        .assert()
        .failure()
        .stderr(contains("error: unexpected argument '--backend'"));
}
