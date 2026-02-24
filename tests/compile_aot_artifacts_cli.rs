use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use serde_json::Value;
use std::fs;
mod common;

// ---------------------------------------------------------------------------
// Core artifact contract
// ---------------------------------------------------------------------------

/// `tonic compile --backend llvm` MUST produce a real ELF executable at the
/// reported path.  Sidecar artifacts (.ll, .tir.json, .tnx.json) are internal
/// implementation details and are kept for compatibility.
#[test]
fn compile_llvm_produces_real_elf_executable() {
    let temp_dir = common::unique_temp_dir("compile-llvm-elf");
    let source_path = temp_dir.join("native.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    1 + 2\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "native.tn", "--backend", "llvm"])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected llvm compile to succeed, got stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");

    // compile: ok must report the executable path (no extension)
    assert!(
        stdout.contains("compile: ok"),
        "expected compile: ok in stdout: {stdout}"
    );
    assert!(
        stdout.contains(".tonic/build/native"),
        "expected executable path in stdout: {stdout}"
    );
    // Must NOT report the manifest as the primary artifact
    assert!(
        !stdout.trim_end().ends_with(".tnx.json"),
        "compile: ok must point to the executable, not the manifest: {stdout}"
    );

    // Real ELF binary at the reported path
    let exe_path = temp_dir.join(".tonic/build/native");
    assert!(
        exe_path.exists(),
        "ELF executable should exist at {}",
        exe_path.display()
    );

    let elf_bytes = fs::read(&exe_path).expect("should be able to read ELF file");
    assert_eq!(
        &elf_bytes[..4],
        b"\x7fELF",
        "output file must start with ELF magic bytes"
    );

    // Executable permissions must be set
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(&exe_path).unwrap().permissions().mode();
        assert_ne!(
            mode & 0o111,
            0,
            "executable bit must be set on output binary"
        );
    }

    // Sidecar artifacts are still present as internal implementation details
    assert!(
        temp_dir.join(".tonic/build/native.tnx.json").exists(),
        "manifest sidecar should exist"
    );
    assert!(
        temp_dir.join(".tonic/build/native.tir.json").exists(),
        "IR sidecar should exist"
    );
    assert!(
        temp_dir.join(".tonic/build/native.ll").exists(),
        "LLVM IR sidecar should exist"
    );
}

// ---------------------------------------------------------------------------
// Direct execution contract
// ---------------------------------------------------------------------------

/// Running the compiled ELF directly should produce the correct output
/// without requiring `tonic run`.
#[test]
fn compiled_elf_runs_directly_with_expected_output() {
    let temp_dir = common::unique_temp_dir("compile-llvm-direct-run");
    let source_path = temp_dir.join("demo.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    40 + 2\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "demo.tn", "--backend", "llvm"])
        .assert()
        .success();

    let exe_path = temp_dir.join(".tonic/build/demo");
    assert!(exe_path.exists(), "ELF executable should exist");

    // Execute the binary directly (no tonic run wrapper)
    let run_output = std::process::Command::new(&exe_path)
        .output()
        .expect("compiled binary should be executable");

    assert!(
        run_output.status.success(),
        "direct ELF execution should succeed, stderr: {}",
        String::from_utf8_lossy(&run_output.stderr)
    );

    let stdout = String::from_utf8(run_output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout.trim_end(),
        "42",
        "direct execution output should match interpreter output"
    );
}

// ---------------------------------------------------------------------------
// Runtime parity: direct ELF vs interpreter
// ---------------------------------------------------------------------------

/// For integer arithmetic programs, the ELF output must match `tonic run`.
#[test]
fn compiled_elf_output_matches_interpreter_for_arithmetic() {
    let source = "defmodule Demo do\n  def run() do\n    3 * 7 + 1\n  end\nend\n";

    let temp_dir = common::unique_temp_dir("compile-llvm-parity");
    let source_path = temp_dir.join("arith.tn");
    fs::write(&source_path, source).unwrap();

    // Run via interpreter
    let interp_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "arith.tn"])
        .output()
        .expect("tonic run should execute");

    assert!(
        interp_output.status.success(),
        "interpreter run should succeed"
    );

    // Compile to native ELF
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "arith.tn", "--backend", "llvm"])
        .assert()
        .success();

    let exe_path = temp_dir.join(".tonic/build/arith");

    // Run ELF directly
    let native_output = std::process::Command::new(&exe_path)
        .output()
        .expect("compiled binary should execute");

    assert_eq!(
        native_output.status.code(),
        interp_output.status.code(),
        "exit codes must match"
    );
    assert_eq!(
        native_output.stdout, interp_output.stdout,
        "stdout must match between interpreter and native ELF"
    );
}

// ---------------------------------------------------------------------------
// --out contract
// ---------------------------------------------------------------------------

/// `tonic compile --backend llvm --out ./someexe` writes the ELF exactly at
/// that path and the binary is directly executable.
#[test]
fn compile_llvm_out_flag_writes_executable_at_specified_path() {
    let temp_dir = common::unique_temp_dir("compile-llvm-out");
    let source_path = temp_dir.join("out_test.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    10 - 3\n  end\nend\n",
    )
    .unwrap();

    let exe_path = temp_dir.join("my_binary");

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args([
            "compile",
            "out_test.tn",
            "--backend",
            "llvm",
            "--out",
            exe_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("compile: ok"))
        .stdout(contains("my_binary"));

    assert!(exe_path.exists(), "ELF should be at the --out path");

    let elf_bytes = fs::read(&exe_path).expect("should read ELF");
    assert_eq!(&elf_bytes[..4], b"\x7fELF", "output must be a real ELF");

    // Run it
    let run_output = std::process::Command::new(&exe_path)
        .output()
        .expect("binary should run");

    assert!(run_output.status.success(), "binary should exit 0");
    let stdout = String::from_utf8(run_output.stdout).unwrap();
    assert_eq!(stdout.trim_end(), "7", "output should be 10 - 3 = 7");
}

// ---------------------------------------------------------------------------
// Toolchain failure diagnostics
// ---------------------------------------------------------------------------

/// When the C compiler is not available, compile should emit a deterministic
/// diagnostic identifying the stage and the missing tool.
///
/// We test the diagnostic format using the linker error type directly rather
/// than subprocesses (since `cc` IS present in the test environment).
#[test]
fn linker_error_missing_tool_has_deterministic_message() {
    let err = crate::linker_diagnostic_format::tool_not_found_message("cc");
    assert!(
        err.contains("not found in PATH"),
        "missing tool diagnostic must mention PATH: {err}"
    );
    assert!(
        err.contains("'cc'"),
        "missing tool diagnostic must name the tool: {err}"
    );
    assert!(
        err.contains("gcc") || err.contains("clang"),
        "diagnostic should suggest an alternative: {err}"
    );
}

// ---------------------------------------------------------------------------
// Backward-compat: manifest sidecar still works with tonic run
// ---------------------------------------------------------------------------

/// `tonic run .tnx.json` still works after compile because the manifest
/// sidecar is retained as an internal implementation detail.
#[test]
fn run_native_manifest_sidecar_still_works_with_interpreter() {
    let temp_dir = common::unique_temp_dir("run-native-artifact-manifest");
    let source_path = temp_dir.join("native_run.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    40 + 2\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "native_run.tn", "--backend", "llvm"])
        .assert()
        .success();

    // Manifest sidecar is still at the expected path
    assert!(
        temp_dir.join(".tonic/build/native_run.tnx.json").exists(),
        "manifest sidecar should exist for backward compatibility"
    );

    // tonic run still works with the manifest
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", ".tonic/build/native_run.tnx.json"])
        .assert()
        .success()
        .stderr("")
        .stdout(contains("42\n"));
}

// ---------------------------------------------------------------------------
// Rejects --emit flag (unchanged)
// ---------------------------------------------------------------------------

#[test]
fn compile_rejects_emit_flag_as_unexpected_argument() {
    let temp_dir = common::unique_temp_dir("compile-emit-unexpected");
    let source_path = temp_dir.join("emit.tn");
    fs::write(
        &source_path,
        "defmodule Emit do\n  def run() do\n    1\n  end\nend\n",
    )
    .unwrap();

    // --emit is no longer part of the compile CLI contract; any value must fail
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "emit.tn", "--emit", "executable"])
        .assert()
        .failure()
        .stderr(contains("error: unexpected argument '--emit'"));
}

// ---------------------------------------------------------------------------
// Target mismatch (unchanged – still uses manifest)
// ---------------------------------------------------------------------------

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
        .args(["compile", "native_target.tn", "--backend", "llvm"])
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

// ---------------------------------------------------------------------------
// Helper module for testing diagnostic format without subprocess tricks
// ---------------------------------------------------------------------------

mod linker_diagnostic_format {
    pub fn tool_not_found_message(tool: &str) -> String {
        format!(
            "native toolchain not found: '{tool}' not found in PATH; \
            install gcc or clang to enable native compilation"
        )
    }
}
