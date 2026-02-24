use std::fs;

mod common;

#[test]
fn llvm_compile_profiles_phases_and_folds_constant_int_ops() {
    let temp_dir = common::unique_temp_dir("llvm-optimization-profile");
    let source_path = temp_dir.join("demo.tn");
    let profile_path = temp_dir.join("profiles/compile-run.jsonl");
    let artifact_base = temp_dir.join("build/demo");

    fs::create_dir_all(
        artifact_base
            .parent()
            .expect("artifact parent should exist"),
    )
    .unwrap();
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    1 + 2\n  end\nend\n",
    )
    .unwrap();

    let compile_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .env("TONIC_PROFILE_OUT", &profile_path)
        .args([
            "compile",
            "demo.tn",
            "--backend",
            "llvm",
            "--out",
            artifact_base
                .to_str()
                .expect("artifact path should be utf8"),
        ])
        .output()
        .expect("compile command should execute");

    assert!(
        compile_output.status.success(),
        "expected llvm compile to succeed, got stderr: {}",
        String::from_utf8_lossy(&compile_output.stderr)
    );

    // Sidecar artifacts always land in .tonic/build/ regardless of --out path.
    let llvm_ir = fs::read_to_string(temp_dir.join(".tonic/build/demo.ll"))
        .expect("llvm ir sidecar should exist at default build location");

    assert!(
        llvm_ir.contains("%v2 = add i64 0, 3"),
        "expected folded constant result in llvm ir, got:\n{}",
        llvm_ir
    );
    assert!(
        !llvm_ir.contains("%v2 = add i64 %v0, %v1"),
        "expected constant folding to remove runtime add instruction, got:\n{}",
        llvm_ir
    );

    let run_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .env("TONIC_PROFILE_OUT", &profile_path)
        .args(["run", "demo.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        run_output.status.success(),
        "expected run to succeed, got stderr: {}",
        String::from_utf8_lossy(&run_output.stderr)
    );

    let payload = fs::read_to_string(&profile_path).expect("profile output should exist");
    let entries = payload
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).expect("profile line json"))
        .collect::<Vec<_>>();

    assert_eq!(entries.len(), 2, "expected compile + run profile entries");

    let compile_profile = entries
        .iter()
        .find(|entry| entry["command"] == "compile")
        .expect("compile profile entry should exist");
    let compile_phases = compile_profile["phases"]
        .as_array()
        .expect("compile phases should be array");

    assert!(
        compile_phases
            .iter()
            .any(|phase| phase["name"] == "backend.optimize_mir"),
        "expected optimize phase in compile profile: {compile_profile}"
    );

    let run_profile = entries
        .iter()
        .find(|entry| entry["command"] == "run")
        .expect("run profile entry should exist");
    let run_phases = run_profile["phases"]
        .as_array()
        .expect("run phases should be array");

    assert!(
        run_phases
            .iter()
            .any(|phase| phase["name"] == "run.evaluate_entrypoint"),
        "expected runtime phase in run profile: {run_profile}"
    );
}
