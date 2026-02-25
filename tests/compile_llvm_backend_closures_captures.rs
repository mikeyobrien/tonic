use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_llvm_backend_lowers_closure_creation_and_invocation_helpers() {
    let temp_dir = common::unique_temp_dir("compile-llvm-closures-captures");
    let source_path = temp_dir.join("closures.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def make_adder(base) do\n    fn value -> value + base end\n  end\n\n  def capture_short() do\n    (&(&1 + 1)).(41)\n  end\n\n  def run() do\n    tuple(make_adder(4).(3), capture_short())\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "closures.tn"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let ll_path = temp_dir.join(".tonic/build/closures.ll");
    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");

    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_closure(i64, i64, i64)"));
    assert!(llvm_ir.contains("declare i64 (i64, i64, ...) @tn_runtime_call_closure"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_closure"));
    assert!(llvm_ir.contains("call i64 (i64, i64, ...) @tn_runtime_call_closure"));
}
