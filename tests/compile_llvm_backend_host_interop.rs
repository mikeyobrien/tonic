use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_llvm_backend_lowers_host_call_and_protocol_dispatch_helpers() {
    let temp_dir = common::unique_temp_dir("compile-llvm-host-interop");
    let source_path = temp_dir.join("host_interop.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def host_sum(a, b) do\n    host_call(:sum_ints, a, b)\n  end\n\n  def dispatch_tuple() do\n    protocol_dispatch(tuple(1, 2))\n  end\n\n  def run() do\n    tuple(host_sum(1, 2), dispatch_tuple())\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "host_interop.tn", "--backend", "llvm"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let ll_path = temp_dir.join(".tonic/build/host_interop.ll");
    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");

    assert!(llvm_ir.contains("declare i64 (i64, ...) @tn_runtime_host_call"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_protocol_dispatch(i64)"));
    assert!(llvm_ir.contains("call i64 (i64, ...) @tn_runtime_host_call"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_protocol_dispatch"));
}
