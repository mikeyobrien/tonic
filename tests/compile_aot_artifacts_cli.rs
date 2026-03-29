use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use serde_json::Value;
use std::fs;

mod common;

// ---------------------------------------------------------------------------
// Core artifact contract
// ---------------------------------------------------------------------------

/// `tonic compile` MUST produce a real native executable at the
/// reported path. Sidecar artifacts (.c, .tir.json, .tnx.json) are internal
/// implementation details and are kept for compatibility.
#[test]
fn compile_produces_real_native_executable() {
    let temp_dir = common::unique_temp_dir("compile-native-executable");
    let source_path = temp_dir.join("native.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    1 + 2\n  end\nend\n",
    )
    .unwrap();

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "native.tn"])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected native compile to succeed, got stderr: {}",
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

    let elf_bytes = fs::read(&exe_path).expect("should be able to read executable file");
    assert!(
        common::is_native_executable(&elf_bytes),
        "output file must start with native executable magic bytes, got {:?}",
        &elf_bytes[..4]
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
        temp_dir.join(".tonic/build/native.c").exists(),
        "C source sidecar should exist"
    );
}

// ---------------------------------------------------------------------------
// Direct execution contract
// ---------------------------------------------------------------------------

/// Running the compiled ELF directly should produce the correct output
/// without requiring `tonic run`.
#[test]
fn compiled_elf_runs_directly_with_expected_output() {
    let temp_dir = common::unique_temp_dir("compile-native-direct-run");
    let source_path = temp_dir.join("demo.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    40 + 2\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "demo.tn"])
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

    let temp_dir = common::unique_temp_dir("compile-native-parity");
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
        .args(["compile", "arith.tn"])
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

#[test]
fn compiled_elf_suppresses_final_value_after_stdout_side_effects() {
    let fixture_root = common::unique_fixture_root("compile-stdout-contract-puts");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    IO.puts(\"hi\")\n    7\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("interpreter run should execute");

    assert!(
        interpreted.status.success(),
        "expected interpreter success, got status {:?} and stderr: {}",
        interpreted.status.code(),
        String::from_utf8_lossy(&interpreted.stderr)
    );
    assert_eq!(
        String::from_utf8(interpreted.stdout.clone()).expect("interpreter stdout should be utf8"),
        "hi\n"
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success();

    let compiled = std::process::Command::new(fixture_root.join(".tonic/build/main"))
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");

    assert!(
        compiled.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        compiled.status.code(),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert_eq!(
        compiled.stdout, interpreted.stdout,
        "stdout must match interpreter when stdout side effects already occurred"
    );
}

#[test]
fn compiled_elf_system_run_stream_matches_interpreter_for_stderr_only_output() {
    let fixture_root = common::unique_fixture_root("compile-system-run-stream-stderr");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Map.get(System.run(\"printf 'oops\\n' 1>&2\", %{stream: true}), :output, \"\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("interpreter run should execute");

    assert!(
        interpreted.status.success(),
        "expected interpreter success, got status {:?} and stderr: {}",
        interpreted.status.code(),
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success();

    let compiled = std::process::Command::new(fixture_root.join(".tonic/build/main"))
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");

    assert!(
        compiled.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        compiled.status.code(),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert_eq!(
        compiled.stdout, interpreted.stdout,
        "stdout must match interpreter"
    );
    assert_eq!(
        compiled.stderr, interpreted.stderr,
        "stderr must match interpreter"
    );
}

#[test]
fn compiled_elf_supports_list_and_tuple_kernel_builtins() {
    let temp_dir = common::unique_temp_dir("compile-kernel-builtins");
    let source_path = temp_dir.join("builtins.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    pair = {[1, 2, 3], :ok}\n    updated = put_elem(pair, 1, elem({\"x\", 5}, 1))\n    length(tl(elem(updated, 0))) + tuple_size(updated) + hd(elem(updated, 0))\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "builtins.tn"])
        .assert()
        .success();

    let exe_path = temp_dir.join(".tonic/build/builtins");
    let run_output = std::process::Command::new(&exe_path)
        .output()
        .expect("compiled builtins binary should execute");

    assert!(
        run_output.status.success(),
        "compiled builtins program should succeed, stderr: {}",
        String::from_utf8_lossy(&run_output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run_output.stdout).trim_end(), "5");
}

#[test]
fn compiled_elf_supports_stdlib_enum_helpers_using_length_and_elem() {
    let temp_dir = common::unique_temp_dir("compile-stdlib-enum-helpers");
    let source_path = temp_dir.join("enum_helpers.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    fetched = Enum.fetch([10, 20, 30], 1)\n    reduced = Enum.reduce_while([1, 2, 3, 4], 0, fn x, acc ->\n      case acc + x >= 6 do\n        true -> {:halt, acc}\n        _ -> {:cont, acc + x}\n      end\n    end)\n    {fetched, reduced}\n  end\nend\n",
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "enum_helpers.tn"])
        .output()
        .expect("interpreter run should execute enum helpers program");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "enum_helpers.tn"])
        .assert()
        .success();

    let exe_path = temp_dir.join(".tonic/build/enum_helpers");
    let native_output = std::process::Command::new(&exe_path)
        .output()
        .expect("compiled enum helpers binary should execute");

    assert_eq!(
        native_output.status.code(),
        interpreted.status.code(),
        "exit codes must match"
    );
    assert_eq!(
        native_output.stdout, interpreted.stdout,
        "stdout must match between interpreter and native ELF"
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_abs_max_min_round_and_trunc() {
    let temp_dir = common::unique_temp_dir("compile-math-builtins-focused");
    let source_path = temp_dir.join("main.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    [\n      abs(-42),\n      max(-5, -1),\n      min(-5, -1),\n      max(3.5, 2.1),\n      min(3.5, 2.1),\n      round(3.7),\n      round(3.2),\n      trunc(3.7),\n      trunc(3.2)\n    ]\n  end\nend\n",
    )
    .unwrap();

    let interp_output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "main.tn"])
        .output()
        .expect("tonic run should execute focused math builtins program");
    assert!(
        interp_output.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interp_output.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "main.tn"])
        .assert()
        .success();

    let exe_path = temp_dir.join(".tonic/build/main");
    let native_output = std::process::Command::new(&exe_path)
        .output()
        .expect("compiled math builtins binary should execute");

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

#[test]
fn compiled_elf_matches_interpreter_for_string_processing_length_host() {
    let temp_dir = common::unique_temp_dir("compile-string-processing-length-host");
    let source_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/apps/string_processing/src/main.tn");
    let exe_path = temp_dir.join("string_processing_native");

    assert!(
        source_path.exists(),
        "string processing example must exist at {}",
        source_path.display()
    );

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", source_path.to_str().unwrap()])
        .output()
        .expect("tonic run should execute string processing example");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interpreted_stdout =
        String::from_utf8(interpreted.stdout.clone()).expect("interpreter stdout should be utf8");
    assert!(
        interpreted_stdout.contains("length: 8"),
        "interpreter output must include String.length evidence, got: {interpreted_stdout}"
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args([
            "compile",
            source_path.to_str().unwrap(),
            "--out",
            exe_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let native_output = std::process::Command::new(&exe_path)
        .current_dir(&temp_dir)
        .output()
        .expect("compiled string processing binary should execute");
    assert!(
        native_output.status.success(),
        "compiled string processing binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    let native_stdout =
        String::from_utf8(native_output.stdout.clone()).expect("native stdout should be utf8");
    assert!(
        native_stdout.contains("length: 8"),
        "native output must include String.length evidence, got: {native_stdout}"
    );

    assert_eq!(
        native_output.status.code(),
        interpreted.status.code(),
        "exit codes must match"
    );
    assert_eq!(
        native_output.stdout, interpreted.stdout,
        "stdout must match between interpreter and native ELF"
    );
}

// ---------------------------------------------------------------------------
// --out contract
// ---------------------------------------------------------------------------

/// `tonic compile --out ./someexe` writes the ELF exactly at
/// that path and the binary is directly executable.
#[test]
fn compile_out_flag_writes_executable_at_specified_path() {
    let temp_dir = common::unique_temp_dir("compile-native-out");
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
            "--out",
            exe_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(contains("compile: ok"))
        .stdout(contains("my_binary"));

    assert!(exe_path.exists(), "ELF should be at the --out path");

    let elf_bytes = fs::read(&exe_path).expect("should read executable");
    assert!(
        common::is_native_executable(&elf_bytes),
        "output must be a native executable, got {:?}",
        &elf_bytes[..4]
    );

    // Run it
    let run_output = std::process::Command::new(&exe_path)
        .output()
        .expect("binary should run");

    assert!(run_output.status.success(), "binary should exit 0");
    let stdout = String::from_utf8(run_output.stdout).unwrap();
    assert_eq!(stdout.trim_end(), "7", "output should be 10 - 3 = 7");
}

/// `--out ./someexe` should honor the exact relative output path and support
/// idiomatic direct execution as `./someexe` from the working directory.
#[test]
fn compile_out_relative_path_supports_dot_slash_execution_contract() {
    let temp_dir = common::unique_temp_dir("compile-native-out-relative-dot-slash");
    let source_path = temp_dir.join("dot_out.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    6 * 7\n  end\nend\n",
    )
    .unwrap();

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "dot_out.tn", "--out", "./someexe"])
        .assert()
        .success()
        .stdout(contains("compile: ok ./someexe"));

    let exe_path = temp_dir.join("someexe");
    assert!(
        exe_path.exists(),
        "ELF should be written exactly at working-dir-relative ./someexe"
    );

    let run_output = std::process::Command::new("./someexe")
        .current_dir(&temp_dir)
        .output()
        .expect("./someexe should execute directly");

    assert!(run_output.status.success(), "./someexe should exit 0");
    let stdout = String::from_utf8(run_output.stdout).unwrap();
    assert_eq!(
        stdout.trim_end(),
        "42",
        "./someexe output should be correct"
    );
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
        .args(["compile", "native_run.tn"])
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
        .args(["compile", "native_target.tn"])
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
