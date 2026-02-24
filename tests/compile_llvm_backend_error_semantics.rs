use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_llvm_backend_lowers_result_question_raise_and_try_helpers() {
    let temp_dir = common::unique_temp_dir("compile-llvm-error-semantics");
    let source_path = temp_dir.join("error_semantics.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def wrap_ok() do\n    ok(:ok)\n  end\n\n  def bubble_err() do\n    err(:boom)?\n  end\n\n  def raise_direct() do\n    raise(:boom)\n  end\n\n  def run() do\n    try do\n      raise(:boom)\n    rescue\n      :boom -> :rescued\n    after\n      :cleanup\n    end\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "error_semantics.tn", "--backend", "llvm"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let ll_path = temp_dir.join(".tonic/build/error_semantics.ll");
    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");

    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_ok(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_err(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_question(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_raise(i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_try(i64)"));

    assert!(llvm_ir.contains("call i64 @tn_runtime_make_ok"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_err"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_question"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_raise"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_try"));
}
