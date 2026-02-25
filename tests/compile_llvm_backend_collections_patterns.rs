use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_llvm_backend_lowers_collection_builtins_and_pattern_matching_helpers() {
    let temp_dir = common::unique_temp_dir("compile-llvm-collections-patterns");
    let source_path = temp_dir.join("collections_patterns.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def match_list() do\n    [head, _] = list(9, 4)\n  end\n\n  def classify(expected) do\n    case map(:ok, list(expected, 8)) do\n      %{:ok => [^expected, value]} when value == 8 -> value\n      _ -> 0\n    end\n  end\n\n  def run() do\n    tuple(match_list(), classify(7))\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "collections_patterns.tn"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let ll_path = temp_dir.join(".tonic/build/collections_patterns.ll");
    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");

    assert!(llvm_ir.contains("declare i64 (i64, ...) @tn_runtime_make_list"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_map(i64, i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_make_tuple(i64, i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_load_binding(i64)"));
    assert!(llvm_ir.contains("declare i1 @tn_runtime_pattern_matches(i64, i64)"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_match_operator(i64, i64)"));

    assert!(llvm_ir.contains("call i64 (i64, ...) @tn_runtime_make_list"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_map"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_make_tuple"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_load_binding"));
    assert!(llvm_ir.contains("call i1 @tn_runtime_pattern_matches"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_match_operator"));
}

#[test]
fn compile_llvm_backend_keeps_deterministic_mismatch_helpers_for_case_and_match() {
    let temp_dir = common::unique_temp_dir("compile-llvm-pattern-mismatch-helpers");
    let source_path = temp_dir.join("mismatch_helpers.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def no_clause(value) do\n    case value do\n      _ when false -> 1\n    end\n  end\n\n  def bad_match() do\n    [1, 2] = list(1, 3)\n  end\n\n  def run() do\n    tuple(no_clause(list(1)), bad_match())\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "mismatch_helpers.tn"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let ll_path = temp_dir.join(".tonic/build/mismatch_helpers.ll");
    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");

    assert!(llvm_ir.contains("declare i64 @tn_runtime_error_no_matching_clause()"));
    assert!(llvm_ir.contains("declare i64 @tn_runtime_error_bad_match()"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_error_no_matching_clause()"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_match_operator"));
}
