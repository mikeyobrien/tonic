use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
mod common;

#[test]
fn compile_llvm_backend_lowers_control_flow_dispatch_and_main_entrypoint() {
    let temp_dir = common::unique_temp_dir("compile-llvm-control-flow");
    let source_path = temp_dir.join("control_flow.tn");
    fs::write(
        &source_path,
        "defmodule Math do\n  def choose(value) when value > 10 do\n    value\n  end\n\n  def choose(value) do\n    value + 1\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    if Math.choose(7) > 8 do\n      1\n    else\n      0\n    end\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "control_flow.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected llvm compile to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let ll_path = temp_dir.join(".tonic/build/control_flow.ll");
    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");

    assert!(llvm_ir.contains("define i64 @tn_Math_choose__arity1(i64 %arg0)"));
    assert!(llvm_ir.contains("define i64 @tn_Math_choose__arity1__clause0(i64 %arg0)"));
    assert!(llvm_ir.contains("define i64 @tn_Math_choose__arity1__clause1(i64 %arg0)"));
    assert!(llvm_ir.contains("call i64 @tn_Math_choose__arity1(i64"));
    assert!(llvm_ir.contains("br i1"));
    assert!(llvm_ir.contains("define i64 @main()"));
    assert!(llvm_ir.contains("call i64 @tn_Demo_run__arity0()"));
}

#[test]
fn compile_llvm_backend_emits_deterministic_no_clause_helper_calls() {
    let temp_dir = common::unique_temp_dir("compile-llvm-no-clause");
    let source_path = temp_dir.join("no_clause.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def choose(value) when value == 7 do\n    value\n  end\n\n  def run() do\n    choose(7)\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "no_clause.tn"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("compile: ok"));

    let ll_path = temp_dir.join(".tonic/build/no_clause.ll");
    let llvm_ir = fs::read_to_string(&ll_path).expect("llvm ir artifact should be readable");

    assert!(llvm_ir.contains("declare i64 @tn_runtime_error_no_matching_clause()"));
    assert!(llvm_ir.contains("call i64 @tn_runtime_error_no_matching_clause()"));
}

#[test]
fn compile_llvm_backend_handles_control_flow_catalog_cfg_fixtures() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("compile-llvm-control-flow-catalog");

    for fixture in [
        "examples/parity/02-operators/logical_keywords.tn",
        "examples/parity/02-operators/logical_short_circuit.tn",
        "examples/parity/06-control-flow/if_unless.tn",
    ] {
        let source = repo_root.join(fixture);
        assert!(source.exists(), "expected fixture {fixture} to exist");

        let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
            .current_dir(&temp_dir)
            .args([
                "compile",
                source.to_str().expect("fixture path should be utf8"),
            ])
            .output()
            .expect("compile command should execute");

        assert!(
            output.status.success(),
            "expected llvm compile success for {fixture}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn compile_llvm_backend_handles_for_catalog_fixtures_with_reduce_and_into_variants() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("compile-llvm-for-catalog");

    for fixture in [
        "examples/parity/06-control-flow/for_single_generator.tn",
        "examples/parity/06-control-flow/for_multi_generator.tn",
        "examples/parity/06-control-flow/for_into.tn",
        "examples/parity/06-control-flow/for_into_runtime_fail.tn",
        "examples/parity/06-control-flow/for_reduce.tn",
    ] {
        let source = repo_root.join(fixture);
        assert!(source.exists(), "expected fixture {fixture} to exist");

        let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
            .current_dir(&temp_dir)
            .args([
                "compile",
                source.to_str().expect("fixture path should be utf8"),
            ])
            .output()
            .expect("compile command should execute");

        assert!(
            output.status.success(),
            "expected llvm compile success for {fixture}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
            output.status.code(),
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let invalid_source = repo_root.join("examples/parity/06-control-flow/for_reduce_fail.tn");
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args([
            "compile",
            invalid_source
                .to_str()
                .expect("fixture path should be utf8"),
        ])
        .assert()
        .failure()
        .stderr(contains(
            "error: for options 'reduce' and 'into' cannot be combined",
        ));
}
