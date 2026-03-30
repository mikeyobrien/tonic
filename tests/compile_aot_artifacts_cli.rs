use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use serde_json::Value;
use std::fs;
use std::time::{Duration, Instant};

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
fn compiled_elf_matches_interpreter_for_native_boolean_negation() {
    let temp_dir = common::unique_temp_dir("compile-native-boolean-negation");

    let success_path = temp_dir.join("boolean_negation.tn");
    fs::write(
        &success_path,
        "defmodule Demo do\n  def run() do\n    flag = false\n    missing = nil\n    truthy = :ok\n    [not flag, !missing, !flag, !truthy]\n  end\nend\n",
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "boolean_negation.tn"])
        .output()
        .expect("interpreter run should execute boolean negation program");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&interpreted.stdout).trim_end(),
        "[true, true, true, false]"
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "boolean_negation.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/boolean_negation"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled boolean negation binary should execute");
    assert!(
        native_output.status.success(),
        "compiled boolean negation binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
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
    assert_eq!(
        native_output.stderr, interpreted.stderr,
        "stderr must match between interpreter and native ELF"
    );

    let strict_path = temp_dir.join("strict_not_badarg.tn");
    fs::write(
        &strict_path,
        "defmodule Demo do\n  def run() do\n    value = nil\n    not value\n  end\nend\n",
    )
    .unwrap();

    let interpreted_badarg = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "strict_not_badarg.tn"])
        .output()
        .expect("interpreter run should execute strict-not badarg program");
    assert!(
        !interpreted_badarg.status.success(),
        "interpreter strict-not badarg program should fail"
    );
    assert!(
        String::from_utf8_lossy(&interpreted_badarg.stderr).contains("error: badarg"),
        "interpreter strict-not badarg stderr should mention badarg, got: {}",
        String::from_utf8_lossy(&interpreted_badarg.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "strict_not_badarg.tn"])
        .assert()
        .success();

    let native_badarg = std::process::Command::new(temp_dir.join(".tonic/build/strict_not_badarg"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled strict-not badarg binary should execute");
    assert!(
        !native_badarg.status.success(),
        "compiled strict-not badarg binary should fail"
    );
    assert!(
        String::from_utf8_lossy(&native_badarg.stderr).contains("error: badarg"),
        "compiled strict-not badarg stderr should mention badarg, got: {}",
        String::from_utf8_lossy(&native_badarg.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&native_badarg.stderr).contains("tn_runtime_not"),
        "compiled strict-not badarg stderr should not mention missing tn_runtime_not helper, got: {}",
        String::from_utf8_lossy(&native_badarg.stderr)
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
fn compiled_elf_system_run_timeout_matches_interpreter() {
    let fixture_root = common::unique_fixture_root("compile-system-run-timeout");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    result = System.run(\"printf 'before\\n'; sleep 5; printf 'after\\n'\", %{timeout_ms: 150, stream: true})\n    IO.puts(\"exit=#{Map.get(result, :exit_code, -1)}\")\n    IO.puts(\"timed_out=#{Map.get(result, :timed_out, false)}\")\n    IO.puts(\"output=#{Map.get(result, :output, \"\")}\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let interpreted_started = Instant::now();
    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("interpreter run should execute");
    let interpreted_elapsed = interpreted_started.elapsed();

    assert!(
        interpreted.status.success(),
        "expected interpreter success, got status {:?} and stderr: {}",
        interpreted.status.code(),
        String::from_utf8_lossy(&interpreted.stderr)
    );
    assert!(
        interpreted_elapsed < Duration::from_secs(3),
        "expected interpreter timeout to return quickly, got {interpreted_elapsed:?}"
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success();

    let compiled_started = Instant::now();
    let compiled = std::process::Command::new(fixture_root.join(".tonic/build/main"))
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");
    let compiled_elapsed = compiled_started.elapsed();

    assert!(
        compiled.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        compiled.status.code(),
        String::from_utf8_lossy(&compiled.stderr)
    );
    assert!(
        compiled_elapsed < Duration::from_secs(3),
        "expected compiled timeout to return quickly, got {compiled_elapsed:?}"
    );
    assert_eq!(
        compiled.stdout, interpreted.stdout,
        "stdout must match interpreter"
    );
    assert_eq!(
        compiled.stderr, interpreted.stderr,
        "stderr must match interpreter"
    );

    let stdout = String::from_utf8(compiled.stdout.clone()).expect("stdout should be utf8");
    assert!(
        stdout.contains("before\n"),
        "expected pre-timeout output, got: {stdout:?}"
    );
    assert!(
        stdout.contains("exit=124\n"),
        "expected timeout exit code, got: {stdout:?}"
    );
    assert!(
        stdout.contains("timed_out=true\n"),
        "expected timeout flag, got: {stdout:?}"
    );
    assert!(
        stdout.contains("output=before\n"),
        "expected partial captured output, got: {stdout:?}"
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

#[test]
fn compiled_elf_matches_interpreter_for_integer_parse() {
    let temp_dir = common::unique_temp_dir("compile-integer-parse");
    let source_path = temp_dir.join("integer_parse_test.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    case Integer.parse("123abc") do
      {n, rest} -> IO.puts(rest)
      _ -> IO.puts("fail")
    end
    case Integer.parse("456") do
      {n, rest} -> IO.puts("whole")
      _ -> IO.puts("fail")
    end
    case Integer.parse("abc") do
      {n, rest} -> IO.puts("fail")
      _ -> IO.puts("error")
    end
    case Integer.parse("  -42xyz") do
      {n, rest} -> IO.puts(rest)
      _ -> IO.puts("fail")
    end
  end
end
"#,
    )
    .unwrap();
    let exe_path = temp_dir.join("integer_parse_native");

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", source_path.to_str().unwrap()])
        .output()
        .expect("tonic run should execute integer parse example");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interpreted_stdout =
        String::from_utf8(interpreted.stdout.clone()).expect("interpreter stdout should be utf8");
    assert!(
        interpreted_stdout.contains("abc") && interpreted_stdout.contains("error"),
        "interpreter output must include parse results, got: {interpreted_stdout}"
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
        .expect("compiled integer parse binary should execute");
    assert!(
        native_output.status.success(),
        "compiled integer parse binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
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
// IO.puts auto-coercion parity (non-string values)
// ---------------------------------------------------------------------------

/// Native `IO.puts` must auto-coerce non-string values (Int, Float, Bool)
/// identically to the interpreter's `value_to_string()`.
#[test]
fn compiled_elf_matches_interpreter_for_io_puts_auto_coercion() {
    let temp_dir = common::unique_temp_dir("compile-io-puts-coerce");
    let source_path = temp_dir.join("io_coerce.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    IO.puts(2 + 3)
    IO.puts(10 - 4)
    IO.puts(3 * 7)
    IO.puts(true)
    IO.puts(false)
    IO.puts(3 > 2)
    IO.puts(1 < 0)
    IO.puts(:hello)
    IO.puts(nil)
    IO.puts("done")
  end
end
"#,
    )
    .unwrap();
    let exe_path = temp_dir.join("io_coerce_native");

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", source_path.to_str().unwrap()])
        .output()
        .expect("tonic run should execute io coerce example");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
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
        .output()
        .expect("compiled io coerce binary should execute");
    assert!(
        native_output.status.success(),
        "compiled io coerce binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
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

/// Native float arithmetic (add, sub, mul, mixed int/float, comparisons) must
/// produce identical stdout to the interpreter.
#[test]
fn compiled_elf_matches_interpreter_for_float_arithmetic() {
    let temp_dir = common::unique_temp_dir("compile-float-arith");
    let source_path = temp_dir.join("float_arith.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    IO.puts(1.5 + 2.5)
    IO.puts(5.0 - 2.0)
    IO.puts(3.0 * 2.5)
    IO.puts(1 + 2.5)
    IO.puts(2.5 + 1)
    IO.puts(10 - 3.5)
    IO.puts(3 * 2.5)
    IO.puts(1.5 + 2.5 - 1.0)
    IO.puts(3.14 > 2.0)
    IO.puts(1.5 < 2.5)
    IO.puts(2.0 >= 2.0)
    IO.puts(1.0 <= 0.5)
    IO.puts(3 > 2.5)
    IO.puts(2 < 2.5)
    IO.puts(-1.5 + 2.5)
    IO.puts(-3.0 * 2.0)
    IO.puts(2 + 3)
    IO.puts(10 - 4)
    IO.puts(3 * 7)
  end
end
"#,
    )
    .unwrap();
    let exe_path = temp_dir.join("float_arith_native");

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", source_path.to_str().unwrap()])
        .output()
        .expect("tonic run should execute float arith example");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
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
        .output()
        .expect("compiled float arith binary should execute");
    assert!(
        native_output.status.success(),
        "compiled float arith binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );

    assert_eq!(
        native_output.status.code(),
        interpreted.status.code(),
        "exit codes must match"
    );
    assert_eq!(
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout),
        "stdout must match between interpreter and native for float arithmetic"
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_type_checking() {
    let temp_dir = common::unique_temp_dir("compile-type-checking");
    let source_path = temp_dir.join("type_checking.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    IO.puts(is_integer(42))
    IO.puts(is_integer(3.14))
    IO.puts(is_float(3.14))
    IO.puts(is_float(42))
    IO.puts(is_number(42))
    IO.puts(is_number(3.14))
    IO.puts(is_number(:ok))
    IO.puts(is_atom(:ok))
    IO.puts(is_atom("hello"))
    IO.puts(is_binary("hello"))
    IO.puts(is_binary(42))
    IO.puts(is_list([1, 2]))
    IO.puts(is_list(42))
    IO.puts(is_nil(nil))
    IO.puts(is_nil(42))
    IO.puts(is_boolean(true))
    IO.puts(is_boolean(false))
    IO.puts(is_boolean(42))
    t = {1, 2}
    IO.puts(is_tuple(t))
    IO.puts(is_tuple(42))
    m = %{a: 1}
    IO.puts(is_map(m))
    IO.puts(is_map(42))
  end
end
"#,
    )
    .unwrap();
    let exe_path = temp_dir.join("type_checking_native");

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", source_path.to_str().unwrap()])
        .output()
        .expect("tonic run should execute type_checking example");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
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
        .output()
        .expect("compiled type_checking binary should execute");
    assert!(
        native_output.status.success(),
        "compiled type_checking binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );

    assert_eq!(
        native_output.status.code(),
        interpreted.status.code(),
        "exit codes must match"
    );
    assert_eq!(
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout),
        "stdout must match between interpreter and native for type checking builtins"
    );
}

// ---------------------------------------------------------------------------
// Parity: inspect + map_size builtins
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_inspect_and_map_size() {
    let temp_dir = common::unique_temp_dir("compile-inspect-map-size");
    let source_path = temp_dir.join("inspect_map_size.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # inspect various types
    IO.puts(inspect(42))
    IO.puts(inspect(3.14))
    IO.puts(inspect("hello"))
    IO.puts(inspect(true))
    IO.puts(inspect(nil))
    IO.puts(inspect(:ok))
    IO.puts(inspect([1, 2, 3]))
    IO.puts(inspect({:ok, 42}))
    IO.puts(inspect(%{a: 1, b: 2}))

    # inspect in interpolation
    list = [1, 2, 3]
    IO.puts("list is: #{inspect(list)}")

    # inspect empty collections
    IO.puts(inspect([]))
    IO.puts(inspect(%{}))

    # map_size
    IO.puts(map_size(%{}))
    IO.puts(map_size(%{a: 1, b: 2, c: 3}))
    IO.puts(map_size(%{x: 10, y: 20}))
  end
end
"#,
    )
    .unwrap();
    let exe_path = temp_dir.join("inspect_map_size_native");

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", source_path.to_str().unwrap()])
        .output()
        .expect("tonic run should execute inspect_map_size example");
    assert!(
        interpreted.status.success(),
        "interpreter run should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
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
        .output()
        .expect("compiled inspect_map_size binary should execute");
    assert!(
        native_output.status.success(),
        "compiled inspect_map_size binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );

    assert_eq!(
        native_output.status.code(),
        interpreted.status.code(),
        "exit codes must match"
    );
    assert_eq!(
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout),
        "stdout must match between interpreter and native for inspect and map_size"
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
// Experiment 5: builtins inside closures parity
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_builtins_in_closures() {
    let temp_dir = common::unique_temp_dir("compile-closure-builtins");

    let source = temp_dir.join("closure_builtins.tn");
    fs::write(
        &source,
        r#"defmodule Demo do
  def run() do
    nums = [-3, 1, -2, 4]
    mapped_abs = Enum.map(nums, fn x -> abs(x) end)
    IO.puts(inspect(mapped_abs))

    floats = [1.5, 2.7, 3.1]
    mapped_round = Enum.map(floats, fn f -> round(f) end)
    IO.puts(inspect(mapped_round))

    mapped_trunc = Enum.map(floats, fn f -> trunc(f) end)
    IO.puts(inspect(mapped_trunc))

    lists = [[1, 2], [3], [4, 5, 6]]
    mapped_length = Enum.map(lists, fn l -> length(l) end)
    IO.puts(inspect(mapped_length))

    mapped_hd = Enum.map(lists, fn l -> hd(l) end)
    IO.puts(inspect(mapped_hd))

    vals = [1, :hello, "str", nil, true]
    mapped_types = Enum.map(vals, fn v -> is_integer(v) end)
    IO.puts(inspect(mapped_types))

    maxed = Enum.map(nums, fn x -> max(x, 0) end)
    IO.puts(inspect(maxed))

    mined = Enum.map(nums, fn x -> min(x, 0) end)
    IO.puts(inspect(mined))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "closure_builtins.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "closure_builtins.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/closure_builtins"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled closure-builtins binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

// ---------------------------------------------------------------------------
// Math module host dispatch parity
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_math_module() {
    let temp_dir = common::unique_temp_dir("compile-math-module");

    let source = temp_dir.join("math_parity.tn");
    fs::write(
        &source,
        r#"defmodule Demo do
  def run() do
    IO.puts(host_call(:math_pow, 2, 10))
    IO.puts(host_call(:math_abs, -42))
    IO.puts(host_call(:math_min, 5, 3))
    IO.puts(host_call(:math_max, 5, 3))
    IO.puts(host_call(:math_ceil, 2.3))
    IO.puts(host_call(:math_floor, 2.7))
    IO.puts(host_call(:math_round, 2.5))
    IO.puts(host_call(:math_round, 2.4))
    IO.puts(host_call(:math_abs, -3.14))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "math_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "math_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/math_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled math binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_integer_module() {
    let temp_dir = common::unique_temp_dir("compile-integer-module");

    let source = temp_dir.join("integer_parity.tn");
    fs::write(
        &source,
        r#"defmodule Demo do
  def run() do
    # integer_to_string
    IO.puts(host_call(:integer_to_string, 42))
    IO.puts(host_call(:integer_to_string, -7))
    IO.puts(host_call(:integer_to_string, 0))

    # integer_to_string_base
    IO.puts(host_call(:integer_to_string_base, 255, 16))
    IO.puts(host_call(:integer_to_string_base, 10, 2))
    IO.puts(host_call(:integer_to_string_base, -255, 16))
    IO.puts(host_call(:integer_to_string_base, 0, 8))

    # integer_digits
    IO.puts(inspect(host_call(:integer_digits, 1234)))
    IO.puts(inspect(host_call(:integer_digits, 0)))
    IO.puts(inspect(host_call(:integer_digits, -42)))

    # integer_undigits
    IO.puts(host_call(:integer_undigits, [1, 2, 3, 4]))
    IO.puts(host_call(:integer_undigits, [0]))

    # integer_gcd
    IO.puts(host_call(:integer_gcd, 12, 8))
    IO.puts(host_call(:integer_gcd, 7, 13))
    IO.puts(host_call(:integer_gcd, 5, 0))
    IO.puts(host_call(:integer_gcd, -12, 8))

    # integer_is_even / integer_is_odd
    IO.puts(host_call(:integer_is_even, 4))
    IO.puts(host_call(:integer_is_even, 3))
    IO.puts(host_call(:integer_is_odd, 7))
    IO.puts(host_call(:integer_is_odd, 8))

    # integer_pow
    IO.puts(host_call(:integer_pow, 2, 10))
    IO.puts(host_call(:integer_pow, 99, 0))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "integer_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "integer_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/integer_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled integer binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

// ---------------------------------------------------------------------------
// Map module parity
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_map_module() {
    let temp_dir = common::unique_temp_dir("parity-map-module");
    let source_path = temp_dir.join("map_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    m = %{"a" => 1, "b" => 2, "c" => 3}

    # map_keys
    IO.puts(inspect(host_call(:map_keys, m)))

    # map_values
    IO.puts(inspect(host_call(:map_values, m)))

    # map_merge
    IO.puts(inspect(host_call(:map_merge, m, %{"b" => 99, "d" => 4})))

    # map_drop
    IO.puts(inspect(host_call(:map_drop, m, ["a", "c"])))

    # map_take
    IO.puts(inspect(host_call(:map_take, m, ["a", "c"])))

    # map_has_key
    IO.puts(host_call(:map_has_key, m, "b"))
    IO.puts(host_call(:map_has_key, m, "z"))

    # map_get with default
    IO.puts(host_call(:map_get, m, "a", 0))
    IO.puts(host_call(:map_get, m, "z", 42))

    # map_put (new key)
    IO.puts(inspect(host_call(:map_put, m, "d", 4)))
    # map_put (existing key)
    IO.puts(inspect(host_call(:map_put, m, "a", 99)))

    # map_delete
    IO.puts(inspect(host_call(:map_delete, m, "b")))
    IO.puts(inspect(host_call(:map_delete, m, "z")))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "map_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "map_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/map_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled map binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

// ---------------------------------------------------------------------------
// Float module parity
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_float_module() {
    let temp_dir = common::unique_temp_dir("parity-float-module");
    let source_path = temp_dir.join("float_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # float_to_string
    IO.puts(host_call(:float_to_string, 3.14))
    IO.puts(host_call(:float_to_string, 42))

    # float_round
    IO.puts(host_call(:float_round, 3.14159, 2))
    IO.puts(host_call(:float_round, 2.5, 0))

    # float_ceil
    IO.puts(host_call(:float_ceil, 2.1))
    IO.puts(host_call(:float_ceil, -2.9))

    # float_floor
    IO.puts(host_call(:float_floor, 2.9))
    IO.puts(host_call(:float_floor, -2.1))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "float_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "float_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/float_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled float binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

// ---------------------------------------------------------------------------
// Bitwise module parity
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_bitwise_module() {
    let temp_dir = common::unique_temp_dir("parity-bitwise-module");
    let source_path = temp_dir.join("bitwise_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # bitwise_band
    IO.puts(host_call(:bitwise_band, 12, 10))

    # bitwise_bor
    IO.puts(host_call(:bitwise_bor, 12, 10))

    # bitwise_bxor
    IO.puts(host_call(:bitwise_bxor, 12, 10))

    # bitwise_bnot
    IO.puts(host_call(:bitwise_bnot, 0))

    # bitwise_bsl
    IO.puts(host_call(:bitwise_bsl, 1, 4))

    # bitwise_bsr
    IO.puts(host_call(:bitwise_bsr, 16, 4))

    # negative operand
    IO.puts(host_call(:bitwise_band, -1, 255))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "bitwise_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "bitwise_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/bitwise_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled bitwise binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_path_rootname_and_split() {
    let temp_dir = common::unique_temp_dir("parity-path-rootname-split");
    let source_path = temp_dir.join("path_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # path_rootname: strip extension
    IO.puts(host_call(:path_rootname, "/tmp/foo/bar.txt"))

    # path_rootname: no extension
    IO.puts(host_call(:path_rootname, "/tmp/foo/bar"))

    # path_rootname: bare filename
    IO.puts(host_call(:path_rootname, "bar.txt"))

    # path_split: absolute path
    IO.puts(inspect(host_call(:path_split, "/tmp/foo/bar.txt")))

    # path_split: relative path
    IO.puts(inspect(host_call(:path_split, "foo/bar/baz")))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "path_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "path_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/path_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled path binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_hex_encode_decode() {
    let temp_dir = common::unique_temp_dir("parity-hex");
    let source_path = temp_dir.join("hex_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # hex_encode: ASCII string
    IO.puts(host_call(:hex_encode, "hello"))

    # hex_encode: empty string
    IO.puts(host_call(:hex_encode, ""))

    # hex_encode_upper
    IO.puts(host_call(:hex_encode_upper, "hello"))

    # hex_decode: valid lowercase
    IO.puts(inspect(host_call(:hex_decode, "68656c6c6f")))

    # hex_decode: valid uppercase
    IO.puts(inspect(host_call(:hex_decode, "68656C6C6F")))

    # hex_decode: round-trip
    encoded = host_call(:hex_encode, "Tonic!")
    IO.puts(inspect(host_call(:hex_decode, encoded)))

    # hex_decode: odd-length error
    IO.puts(inspect(host_call(:hex_decode, "abc")))

    # hex_decode: invalid char error
    IO.puts(inspect(host_call(:hex_decode, "zz")))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "hex_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "hex_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/hex_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled hex binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_base64_encode_decode() {
    let temp_dir = common::unique_temp_dir("parity-base64");
    let source_path = temp_dir.join("base64_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # base64_encode: simple string
    IO.puts(host_call(:base64_encode, "hello"))

    # base64_encode: empty string
    IO.puts(host_call(:base64_encode, ""))

    # base64_encode: padding (1 byte → YQ==)
    IO.puts(host_call(:base64_encode, "a"))

    # base64_encode: padding (2 bytes → YWI=)
    IO.puts(host_call(:base64_encode, "ab"))

    # base64_decode: simple
    IO.puts(host_call(:base64_decode, "aGVsbG8="))

    # base64_decode: empty
    IO.puts(host_call(:base64_decode, ""))

    # base64_decode: round-trip
    encoded = host_call(:base64_encode, "Tonic lang!")
    IO.puts(host_call(:base64_decode, encoded))

    # base64_url_encode: simple (no padding)
    IO.puts(host_call(:base64_url_encode, "hello"))

    # base64_url_encode: 1 byte (no padding)
    IO.puts(host_call(:base64_url_encode, "a"))

    # base64_url_decode: simple
    IO.puts(host_call(:base64_url_decode, "aGVsbG8"))

    # base64_url_decode: round-trip
    url_enc = host_call(:base64_url_encode, "data+with/special=chars")
    IO.puts(host_call(:base64_url_decode, url_enc))

    # base64_url_encode: empty
    IO.puts(host_call(:base64_url_encode, ""))

    # base64_url_decode: empty
    IO.puts(host_call(:base64_url_decode, ""))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "base64_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "base64_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/base64_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled base64 binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_url_encode_decode() {
    let temp_dir = common::unique_temp_dir("parity-url");
    let source_path = temp_dir.join("url_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # url_encode: simple string with spaces
    IO.puts(host_call(:url_encode, "hello world"))

    # url_encode: unreserved chars pass through (RFC 3986)
    IO.puts(host_call(:url_encode, "hello-world_2.0~test"))

    # url_encode: special chars
    IO.puts(host_call(:url_encode, "a=1&b=2"))

    # url_encode: empty string
    IO.puts(host_call(:url_encode, ""))

    # url_decode: percent-encoded
    IO.puts(host_call(:url_decode, "hello%20world"))

    # url_decode: plus to space
    IO.puts(host_call(:url_decode, "hello+world"))

    # url_decode: empty
    IO.puts(host_call(:url_decode, ""))

    # round-trip
    encoded = host_call(:url_encode, "café & thé")
    IO.puts(host_call(:url_decode, encoded))
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "url_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "url_parity.tn"])
        .assert()
        .success();

    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/url_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled url binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_tuple_enum_datetime() {
    let temp_dir = common::unique_temp_dir("parity-tuple-enum-dt");
    let source_path = temp_dir.join("tuple_enum_dt_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # tuple_to_list: basic
    t = {10, 20}
    IO.puts(inspect(host_call(:tuple_to_list, t)))

    # list_to_tuple: basic
    l = [30, 40]
    IO.puts(inspect(host_call(:list_to_tuple, l)))

    # tuple round-trip
    original = {"hello", 42}
    listed = host_call(:tuple_to_list, original)
    back = host_call(:list_to_tuple, listed)
    IO.puts(inspect(back))

    # enum_sort: integers
    IO.puts(inspect(host_call(:enum_sort, [5, 1, 3, 2, 4])))

    # enum_sort: strings
    IO.puts(inspect(host_call(:enum_sort, ["banana", "apple", "cherry"])))

    # enum_sort: empty
    IO.puts(inspect(host_call(:enum_sort, [])))

    # enum_slice: middle
    IO.puts(inspect(host_call(:enum_slice, [10, 20, 30, 40, 50], 1, 3)))

    # enum_slice: from start
    IO.puts(inspect(host_call(:enum_slice, [1, 2, 3, 4], 0, 2)))

    # enum_slice: past end (clamp)
    IO.puts(inspect(host_call(:enum_slice, [1, 2, 3], 1, 100)))

    # enum_slice: empty result
    IO.puts(inspect(host_call(:enum_slice, [1, 2, 3], 0, 0)))

    # datetime_unix_now: returns an integer > 0
    ts = host_call(:datetime_unix_now)
    IO.puts(ts > 1000000000)

    # datetime_unix_now_ms: returns an integer > unix_now * 1000
    ts_ms = host_call(:datetime_unix_now_ms)
    IO.puts(ts_ms > ts * 1000 - 1000)
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "tuple_enum_dt_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "tuple_enum_dt_parity.tn"])
        .assert()
        .success();

    let native_output =
        std::process::Command::new(temp_dir.join(".tonic/build/tuple_enum_dt_parity"))
            .current_dir(&temp_dir)
            .output()
            .expect("compiled tuple/enum/datetime binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    assert_eq!(
        native_output.stdout,
        interpreted.stdout,
        "stdout must match between interpreter and native: native={}, interp={}",
        String::from_utf8_lossy(&native_output.stdout),
        String::from_utf8_lossy(&interpreted.stdout)
    );
}

#[test]
fn compiled_elf_matches_interpreter_for_random_uuid_shell() {
    let temp_dir = common::unique_temp_dir("parity-random-uuid-shell");
    let source_path = temp_dir.join("random_uuid_shell_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # shell_quote: simple
    IO.puts(host_call(:shell_quote, "hello"))

    # shell_quote: empty
    IO.puts(host_call(:shell_quote, ""))

    # shell_quote: with single quote
    IO.puts(host_call(:shell_quote, "it's"))

    # shell_quote: with spaces and special chars
    IO.puts(host_call(:shell_quote, "hello world $HOME"))

    # shell_join: simple list
    IO.puts(host_call(:shell_join, ["echo", "hello", "world"]))

    # shell_join: empty list
    IO.puts(host_call(:shell_join, []))

    # shell_join: with special chars
    IO.puts(host_call(:shell_join, ["grep", "-r", "it's a $var"]))

    # random_boolean: returns a boolean (true or false)
    rb = host_call(:random_boolean)
    IO.puts(is_boolean(rb))

    # random_float: returns a float string
    rf = host_call(:random_float)
    IO.puts(is_float(rf))

    # random_integer: returns integer in range
    ri = host_call(:random_integer, 1, 10)
    IO.puts(ri >= 1 and ri <= 10)

    # random_integer: single value range
    ri2 = host_call(:random_integer, 42, 42)
    IO.puts(ri2 == 42)

    # uuid_v4: format check (36 chars, 8-4-4-4-12 with dashes)
    u = host_call(:uuid_v4)
    IO.puts(host_call(:str_length, u) == 36)

    # enum_random: returns element from list
    items = [10, 20, 30]
    er = host_call(:enum_random, items)
    IO.puts(er == 10 or er == 20 or er == 30)
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "random_uuid_shell_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interp_stdout = String::from_utf8_lossy(&interpreted.stdout);
    // All lines should be deterministic except we verify structure
    // Shell functions are deterministic so we compare those lines exactly
    let interp_lines: Vec<&str> = interp_stdout.lines().collect();
    // Lines 0-7: shell_quote and shell_join (deterministic)
    assert_eq!(interp_lines[0], "'hello'");
    assert_eq!(interp_lines[1], "''");
    assert_eq!(interp_lines[2], "'it'\"'\"'s'");
    assert_eq!(interp_lines[3], "'hello world $HOME'");
    assert_eq!(interp_lines[4], "'echo' 'hello' 'world'");
    assert_eq!(interp_lines[5], "");
    assert_eq!(interp_lines[6], "'grep' '-r' 'it'\"'\"'s a $var'");
    // Lines 7-12: boolean/float/int/uuid/enum_random checks → all "true"
    for i in 7..13 {
        assert_eq!(
            interp_lines[i], "true",
            "interpreter line {} should be 'true', got '{}'",
            i, interp_lines[i]
        );
    }

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "random_uuid_shell_parity.tn"])
        .assert()
        .success();

    let native_output =
        std::process::Command::new(temp_dir.join(".tonic/build/random_uuid_shell_parity"))
            .current_dir(&temp_dir)
            .output()
            .expect("compiled random/uuid/shell binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native_output.stdout);
    let native_lines: Vec<&str> = native_stdout.lines().collect();
    // Deterministic shell lines must match exactly
    for i in 0..7 {
        assert_eq!(
            native_lines[i], interp_lines[i],
            "native line {} should match interpreter: native='{}', interp='{}'",
            i, native_lines[i], interp_lines[i]
        );
    }
    // Non-deterministic lines: verify all are "true"
    for i in 7..13 {
        assert_eq!(
            native_lines[i], "true",
            "native line {} should be 'true', got '{}'",
            i, native_lines[i]
        );
    }
}

#[test]
fn compiled_elf_matches_interpreter_for_env_shuffle_datetime() {
    let temp_dir = common::unique_temp_dir("parity-env-shuffle-datetime");
    let source_path = temp_dir.join("env_shuffle_datetime_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # env_set / env_has_key / env_delete round-trip
    host_call(:env_set, "_TN_PARITY_TEST_KEY", "hello123")
    IO.puts(host_call(:env_has_key, "_TN_PARITY_TEST_KEY"))
    host_call(:env_delete, "_TN_PARITY_TEST_KEY")
    IO.puts(not host_call(:env_has_key, "_TN_PARITY_TEST_KEY"))

    # env_all returns a map containing PATH
    all = host_call(:env_all)
    IO.puts(is_map(all))

    # enum_shuffle: same elements when sorted, returns a list
    shuffled = host_call(:enum_shuffle, [1, 2, 3, 4, 5])
    IO.puts(host_call(:enum_sort, shuffled) == [1, 2, 3, 4, 5])
    IO.puts(is_list(shuffled))

    # enum_shuffle: empty list
    empty_shuf = host_call(:enum_shuffle, [])
    IO.puts(empty_shuf == [])

    # enum_shuffle: single element
    single_shuf = host_call(:enum_shuffle, [42])
    IO.puts(single_shuf == [42])

    # datetime_utc_now: returns a string of length 20 (YYYY-MM-DDTHH:MM:SSZ)
    dt = host_call(:datetime_utc_now)
    IO.puts(is_binary(dt))
    IO.puts(host_call(:str_length, dt) == 20)
  end
end
"#,
    )
    .unwrap();

    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "env_shuffle_datetime_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interp_stdout = String::from_utf8_lossy(&interpreted.stdout);
    let interp_lines: Vec<&str> = interp_stdout.lines().collect();
    // All lines should be "true"
    assert_eq!(
        interp_lines.len(),
        9,
        "expected 9 output lines, got {}",
        interp_lines.len()
    );
    for (i, line) in interp_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "interpreter line {} should be 'true', got '{}'",
            i, line
        );
    }

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "env_shuffle_datetime_parity.tn"])
        .assert()
        .success();

    let native_output =
        std::process::Command::new(temp_dir.join(".tonic/build/env_shuffle_datetime_parity"))
            .current_dir(&temp_dir)
            .output()
            .expect("compiled env/shuffle/datetime binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native_output.stdout);
    let native_lines: Vec<&str> = native_stdout.lines().collect();
    assert_eq!(
        native_lines.len(),
        9,
        "expected 9 native output lines, got {}",
        native_lines.len()
    );
    for (i, line) in native_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "native line {} should be 'true', got '{}'",
            i, line
        );
    }
}

// ---------------------------------------------------------------------------
// Logger module parity
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_logger_module() {
    let temp_dir = common::unique_temp_dir("parity-logger");
    let source_path = temp_dir.join("logger_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # logger_info at default level (info=1) — should print to stderr
    result1 = host_call(:logger_info, "hello world")
    IO.puts(result1 == :ok)

    # logger_debug at default level — should be suppressed
    result2 = host_call(:logger_debug, "hidden")
    IO.puts(result2 == :ok)

    # logger_error at default level — should print to stderr
    result3 = host_call(:logger_error, "boom")
    IO.puts(result3 == :ok)

    # logger_warn at default level — should print to stderr
    result4 = host_call(:logger_warn, "careful")
    IO.puts(result4 == :ok)

    # set_level / get_level round-trip
    IO.puts(host_call(:logger_get_level) == :info)
    host_call(:logger_set_level, :error)
    IO.puts(host_call(:logger_get_level) == :error)

    # logger_warn should now be suppressed at error level
    result5 = host_call(:logger_warn, "suppressed")
    IO.puts(result5 == :ok)

    # logger_error still shown at error level
    result6 = host_call(:logger_error, "still shown")
    IO.puts(result6 == :ok)
  end
end
"#,
    )
    .unwrap();

    // --- Interpreter ---
    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "logger_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interp_stdout = String::from_utf8_lossy(&interpreted.stdout);
    let interp_lines: Vec<&str> = interp_stdout.lines().collect();
    assert_eq!(
        interp_lines.len(),
        8,
        "expected 8 output lines, got: {:?}",
        interp_lines
    );
    for (i, line) in interp_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "interpreter line {} should be 'true', got '{}'",
            i, line
        );
    }

    // Check interpreter stderr for expected logger output
    let interp_stderr = String::from_utf8_lossy(&interpreted.stderr);
    assert!(
        interp_stderr.contains("[info] hello world"),
        "interpreter stderr should contain '[info] hello world', got: {}",
        interp_stderr
    );
    assert!(
        !interp_stderr.contains("[debug] hidden"),
        "interpreter stderr should NOT contain '[debug] hidden'"
    );
    assert!(
        interp_stderr.contains("[error] boom"),
        "interpreter stderr should contain '[error] boom'"
    );
    assert!(
        interp_stderr.contains("[warn] careful"),
        "interpreter stderr should contain '[warn] careful'"
    );
    assert!(
        !interp_stderr.contains("[warn] suppressed"),
        "interpreter stderr should NOT contain '[warn] suppressed'"
    );
    assert!(
        interp_stderr.contains("[error] still shown"),
        "interpreter stderr should contain '[error] still shown'"
    );

    // --- Compile ---
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "logger_parity.tn"])
        .assert()
        .success();

    // --- Native ---
    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/logger_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled logger binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native_output.stdout);
    let native_lines: Vec<&str> = native_stdout.lines().collect();
    assert_eq!(
        native_lines.len(),
        8,
        "expected 8 native output lines, got: {:?}",
        native_lines
    );
    for (i, line) in native_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "native line {} should be 'true', got '{}'",
            i, line
        );
    }

    // Check native stderr matches interpreter stderr behavior
    let native_stderr = String::from_utf8_lossy(&native_output.stderr);
    assert!(
        native_stderr.contains("[info] hello world"),
        "native stderr should contain '[info] hello world', got: {}",
        native_stderr
    );
    assert!(
        !native_stderr.contains("[debug] hidden"),
        "native stderr should NOT contain '[debug] hidden'"
    );
    assert!(
        native_stderr.contains("[error] boom"),
        "native stderr should contain '[error] boom'"
    );
    assert!(
        native_stderr.contains("[warn] careful"),
        "native stderr should contain '[warn] careful'"
    );
    assert!(
        !native_stderr.contains("[warn] suppressed"),
        "native stderr should NOT contain '[warn] suppressed'"
    );
    assert!(
        native_stderr.contains("[error] still shown"),
        "native stderr should contain '[error] still shown'"
    );
}

// ---------------------------------------------------------------------------
// Experiment 19: File module + URL query + sys_constant_time_eq parity
// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_file_url_query_constant_time_eq() {
    let temp_dir = common::unique_temp_dir("parity-file-url-ctimeq");
    let source_path = temp_dir.join("file_url_ctimeq.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # --- file_cp / file_rename / file_stat ---
    # test_src.txt is pre-created by the test harness

    # file_cp
    cp_result = host_call(:file_cp, "test_src.txt", "test_copy.txt")
    IO.puts(cp_result == :ok)

    # file_stat on the copy
    {stat_tag, stat_map} = host_call(:file_stat, "test_copy.txt")
    IO.puts(stat_tag == :ok)
    IO.puts(stat_map["size"] == 10)
    IO.puts(stat_map["is_file"] == true)
    IO.puts(stat_map["is_dir"] == false)

    # file_rename
    rename_result = host_call(:file_rename, "test_copy.txt", "test_moved.txt")
    IO.puts(rename_result == :ok)

    # stat on moved file should succeed
    {stat2_tag, _} = host_call(:file_stat, "test_moved.txt")
    IO.puts(stat2_tag == :ok)

    # stat on old name should fail
    {stat3_tag, _} = host_call(:file_stat, "test_copy.txt")
    IO.puts(stat3_tag == :error)

    # --- url_encode_query / url_decode_query ---
    encoded = host_call(:url_encode_query, %{"name" => "John Doe", "age" => 30})
    # The query string should contain both pairs
    IO.puts(host_call(:str_contains, encoded, "name=John+Doe"))
    IO.puts(host_call(:str_contains, encoded, "age=30"))
    IO.puts(host_call(:str_contains, encoded, "&"))

    decoded = host_call(:url_decode_query, "color=red&size=42")
    IO.puts(decoded["color"] == "red")
    IO.puts(decoded["size"] == "42")

    # empty query string should return empty map
    empty_map = host_call(:url_decode_query, "")
    IO.puts(empty_map == %{})

    # --- sys_constant_time_eq ---
    IO.puts(host_call(:sys_constant_time_eq, "abc", "abc") == true)
    IO.puts(host_call(:sys_constant_time_eq, "abc", "xyz") == false)
    IO.puts(host_call(:sys_constant_time_eq, "short", "longer") == false)
    IO.puts(host_call(:sys_constant_time_eq, "", "") == true)
  end
end
"#,
    )
    .unwrap();

    // Pre-create test file for file_cp/file_stat tests
    fs::write(temp_dir.join("test_src.txt"), "hello file").unwrap();

    // --- Interpreter ---
    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "file_url_ctimeq.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interp_stdout = String::from_utf8_lossy(&interpreted.stdout);
    let interp_lines: Vec<&str> = interp_stdout.lines().collect();
    assert_eq!(
        interp_lines.len(),
        18,
        "expected 18 output lines, got: {:?}",
        interp_lines
    );
    for (i, line) in interp_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "interpreter line {} should be 'true', got '{}'",
            i, line
        );
    }

    // Clean up test files and re-create source for native run
    let _ = fs::remove_file(temp_dir.join("test_src.txt"));
    let _ = fs::remove_file(temp_dir.join("test_moved.txt"));
    fs::write(temp_dir.join("test_src.txt"), "hello file").unwrap();

    // --- Compile ---
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "file_url_ctimeq.tn"])
        .assert()
        .success();

    // --- Native ---
    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/file_url_ctimeq"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled file_url_ctimeq binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native_output.stdout);
    let native_lines: Vec<&str> = native_stdout.lines().collect();
    assert_eq!(
        native_lines.len(),
        18,
        "expected 18 native output lines, got: {:?}",
        native_lines
    );
    for (i, line) in native_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "native line {} should be 'true', got '{}'",
            i, line
        );
    }
}

// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_assert_module() {
    let temp_dir = common::unique_temp_dir("parity-assert");
    let source_path = temp_dir.join("assert_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # assert — truthy passes
    IO.puts(host_call(:assert, true) == :ok)
    IO.puts(host_call(:assert, 42) == :ok)

    # assert — falsy fails (result err)
    IO.puts(host_call(:assert, false) != :ok)
    IO.puts(host_call(:assert, nil) != :ok)

    # refute — falsy passes
    IO.puts(host_call(:refute, false) == :ok)
    IO.puts(host_call(:refute, nil) == :ok)

    # refute — truthy fails
    IO.puts(host_call(:refute, true) != :ok)

    # assert_equal — equal passes
    IO.puts(host_call(:assert_equal, 42, 42) == :ok)
    IO.puts(host_call(:assert_equal, "hello", "hello") == :ok)

    # assert_equal — not equal fails
    IO.puts(host_call(:assert_equal, 1, 2) != :ok)

    # assert_not_equal — different passes
    IO.puts(host_call(:assert_not_equal, 1, 2) == :ok)

    # assert_not_equal — same fails
    IO.puts(host_call(:assert_not_equal, 1, 1) != :ok)

    # assert_contains — string substring
    IO.puts(host_call(:assert_contains, "hello world", "world") == :ok)

    # assert_contains — list membership
    IO.puts(host_call(:assert_contains, [1, 2, 3], 2) == :ok)

    # assert_contains — not found fails
    IO.puts(host_call(:assert_contains, "hello", "xyz") != :ok)

    # skip — returns err result with test_skipped
    IO.puts(inspect(host_call(:skip, "not ready")) == "err({:test_skipped, \"not ready\"})")

    # assert_raises_check — match passes
    IO.puts(host_call(:assert_raises_check, "key not found: foo", "key not found") == :ok)

    # assert_raises_check — no match fails
    IO.puts(host_call(:assert_raises_check, "something else", "key not found") != :ok)

    # assert_match — equal non-maps
    IO.puts(host_call(:assert_match, 42, 42) == :ok)

    # assert_match — non-equal non-maps fail
    IO.puts(host_call(:assert_match, 1, 2) != :ok)
  end
end
"#,
    )
    .unwrap();

    // --- Interpreter ---
    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "assert_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interp_stdout = String::from_utf8_lossy(&interpreted.stdout);
    let interp_lines: Vec<&str> = interp_stdout.lines().collect();
    assert_eq!(
        interp_lines.len(),
        20,
        "expected 20 output lines, got: {:?}",
        interp_lines
    );
    for (i, line) in interp_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "interpreter line {} should be 'true', got '{}'",
            i, line
        );
    }

    // --- Compile ---
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "assert_parity.tn"])
        .assert()
        .success();

    // --- Native ---
    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/assert_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled assert binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native_output.stdout);
    let native_lines: Vec<&str> = native_stdout.lines().collect();
    assert_eq!(
        native_lines.len(),
        20,
        "expected 20 native output lines, got: {:?}",
        native_lines
    );
    for (i, line) in native_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "native line {} should be 'true', got '{}'",
            i, line
        );
    }
}

// ---------------------------------------------------------------------------

#[test]
fn compiled_elf_matches_interpreter_for_sys_retry_plan() {
    let temp_dir = common::unique_temp_dir("parity-retry-plan");
    let source_path = temp_dir.join("retry_plan_parity.tn");
    fs::write(
        &source_path,
        r#"defmodule Demo do
  def run() do
    # exhausted: attempt >= max_attempts
    result1 = System.retry_plan(500, 3, 3, 1000, 10000, 0, nil)
    IO.puts(result1[:retry] == false)
    IO.puts(result1[:delay_ms] == 0)
    IO.puts(result1[:source] == :exhausted)

    # non_retryable: 200 is not retryable
    result2 = System.retry_plan(200, 1, 5, 1000, 10000, 0, nil)
    IO.puts(result2[:retry] == false)
    IO.puts(result2[:delay_ms] == 0)
    IO.puts(result2[:source] == :non_retryable)

    # backoff: 500 status, attempt 1, base 1000, max 10000, jitter 0
    result3 = System.retry_plan(500, 1, 5, 1000, 10000, 0, nil)
    IO.puts(result3[:retry] == true)
    IO.puts(result3[:delay_ms] == 1000)
    IO.puts(result3[:source] == :backoff)

    # backoff: 502 status, attempt 2, base 1000, max 10000, jitter 0
    result4 = System.retry_plan(502, 2, 5, 1000, 10000, 0, nil)
    IO.puts(result4[:retry] == true)
    IO.puts(result4[:delay_ms] == 2000)
    IO.puts(result4[:source] == :backoff)

    # retry_after: 429 with Retry-After header "3"
    result5 = System.retry_plan(429, 1, 5, 1000, 10000, 0, "3")
    IO.puts(result5[:retry] == true)
    IO.puts(result5[:delay_ms] == 3000)
    IO.puts(result5[:source] == :retry_after)

    # 429 without valid retry_after falls back to backoff
    result6 = System.retry_plan(429, 1, 5, 1000, 10000, 0, nil)
    IO.puts(result6[:retry] == true)
    IO.puts(result6[:source] == :backoff)
  end
end
"#,
    )
    .unwrap();

    // --- Interpreter ---
    let interpreted = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["run", "retry_plan_parity.tn"])
        .output()
        .expect("interpreter run");
    assert!(
        interpreted.status.success(),
        "interpreter should succeed, stderr: {}",
        String::from_utf8_lossy(&interpreted.stderr)
    );
    let interp_stdout = String::from_utf8_lossy(&interpreted.stdout);
    let interp_lines: Vec<&str> = interp_stdout.lines().collect();
    assert_eq!(
        interp_lines.len(),
        17,
        "expected 17 output lines, got: {:?}",
        interp_lines
    );
    for (i, line) in interp_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "interpreter line {} should be 'true', got '{}'",
            i, line
        );
    }

    // --- Compile ---
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&temp_dir)
        .args(["compile", "retry_plan_parity.tn"])
        .assert()
        .success();

    // --- Native ---
    let native_output = std::process::Command::new(temp_dir.join(".tonic/build/retry_plan_parity"))
        .current_dir(&temp_dir)
        .output()
        .expect("compiled retry_plan binary should execute");
    assert!(
        native_output.status.success(),
        "compiled binary should succeed, stderr: {}",
        String::from_utf8_lossy(&native_output.stderr)
    );
    let native_stdout = String::from_utf8_lossy(&native_output.stdout);
    let native_lines: Vec<&str> = native_stdout.lines().collect();
    assert_eq!(
        native_lines.len(),
        17,
        "expected 17 native output lines, got: {:?}",
        native_lines
    );
    for (i, line) in native_lines.iter().enumerate() {
        assert_eq!(
            *line, "true",
            "native line {} should be 'true', got '{}'",
            i, line
        );
    }
}

// ---------------------------------------------------------------------------

mod linker_diagnostic_format {
    pub fn tool_not_found_message(tool: &str) -> String {
        format!(
            "native toolchain not found: '{tool}' not found in PATH; \
            install gcc or clang to enable native compilation"
        )
    }
}
