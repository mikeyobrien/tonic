use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_llvm_backend_emits_ll_and_object_artifacts_for_subset_program() {
    let temp_dir = common::unique_temp_dir("compile-llvm-subset");
    let source_path = temp_dir.join("math.tn");
    fs::write(
        &source_path,
        "defmodule Math do\n  def add(a, b) do\n    a + b\n  end\n\n  def run() do\n    add(7, 8) > 10\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "math.tn", "--backend", "llvm"])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected llvm compile to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("compile: ok"));
    assert!(stdout.contains(".tonic/build/math.ll"));
    assert!(stdout.contains(".tonic/build/math.o"));

    let ll_path = temp_dir.join(".tonic/build/math.ll");
    let object_path = temp_dir.join(".tonic/build/math.o");

    assert!(
        ll_path.exists(),
        "expected .ll artifact at {}",
        ll_path.display()
    );
    assert!(
        object_path.exists(),
        "expected .o artifact at {}",
        object_path.display()
    );

    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");
    assert!(llvm_ir.contains("; tonic llvm backend mvp"));
    assert!(llvm_ir.contains("define i64 @tn_Math_add(i64 %arg0, i64 %arg1)"));
    assert!(llvm_ir.contains("define i64 @tn_Math_run()"));
    assert!(llvm_ir.contains("call i64 @tn_Math_add"));
    assert!(llvm_ir.contains("icmp sgt i64"));

    let object = fs::read(&object_path).expect("object artifact should be readable");
    assert!(
        object.starts_with(b"TONICOBJ"),
        "expected deterministic object placeholder header"
    );
}

#[test]
fn compile_llvm_backend_rejects_unsupported_ops_with_deterministic_diagnostic() {
    let temp_dir = common::unique_temp_dir("compile-llvm-unsupported");
    let source_path = temp_dir.join("unsupported.tn");
    fs::write(
        &source_path,
        "defmodule Unsupported do\n  def run() do\n    \"hello\"\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "unsupported.tn", "--backend", "llvm"])
        .assert()
        .failure()
        .stderr(contains(
            "error: llvm backend unsupported instruction const_string in function Unsupported.run",
        ));
}

#[test]
fn compile_rejects_unknown_backend_value() {
    let temp_dir = common::unique_temp_dir("compile-backend-unknown");
    let source_path = temp_dir.join("single.tn");
    fs::write(
        &source_path,
        "defmodule Single do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "single.tn", "--backend", "nope"])
        .assert()
        .failure()
        .stderr(contains("error: unsupported backend 'nope'"));
}
