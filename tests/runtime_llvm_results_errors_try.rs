mod common;

use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn compiled_llvm_runtime_matches_catalog_for_result_and_try_success_fixtures() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("runtime-llvm-results-errors-try-success");

    for (fixture, expected_stdout) in success_fixture_contracts() {
        let source = repo_root.join(fixture);
        assert!(source.exists(), "expected fixture {fixture} to exist");

        let runtime = compile_and_run_fixture(&temp_dir, &source);

        assert!(
            runtime.status.success(),
            "expected runtime success for {fixture}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
            runtime.status.code(),
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );

        assert_eq!(
            String::from_utf8_lossy(&runtime.stdout),
            expected_stdout,
            "runtime stdout mismatch for {fixture}"
        );
    }
}

#[test]
fn compiled_llvm_runtime_reports_deterministic_result_error_propagation() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("runtime-llvm-results-errors-try-failures");

    for (fixture, expected_stderr_contains) in failure_fixture_contracts() {
        let source = repo_root.join(fixture);
        assert!(source.exists(), "expected fixture {fixture} to exist");

        let runtime = compile_and_run_fixture(&temp_dir, &source);

        assert!(
            !runtime.status.success(),
            "expected runtime failure for {fixture}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
            runtime.status.code(),
            String::from_utf8_lossy(&runtime.stdout),
            String::from_utf8_lossy(&runtime.stderr)
        );

        assert!(
            String::from_utf8_lossy(&runtime.stderr).contains(expected_stderr_contains),
            "expected stderr for {fixture} to contain {expected_stderr_contains:?}, got:\n{}",
            String::from_utf8_lossy(&runtime.stderr)
        );
    }
}

fn success_fixture_contracts() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "examples/parity/08-errors/ok_err_constructors.tn",
            "{ok(42), err(\"failed\")}\n",
        ),
        (
            "examples/parity/08-errors/question_operator_success.tn",
            "15\n",
        ),
        (
            "examples/parity/08-errors/try_rescue_success.tn",
            "\"oops\"\n",
        ),
        (
            "examples/parity/08-errors/try_catch_success.tn",
            "\"caught oops\"\n",
        ),
        ("examples/parity/08-errors/try_after_success.tn", ":ok\n"),
        (
            "examples/parity/08-errors/try_rescue_catch_after_success.tn",
            "\"caught fallback\"\n",
        ),
    ]
}

fn failure_fixture_contracts() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "examples/ergonomics/error_propagation.tn",
            "error: runtime returned err(404)",
        ),
        (
            "examples/parity/08-errors/question_operator_err_bubble.tn",
            "error: runtime returned err(\"bubbly\")",
        ),
    ]
}

fn compile_and_run_fixture(temp_dir: &Path, source: &Path) -> std::process::Output {
    let compile = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(temp_dir)
        .args([
            "compile",
            source.to_str().expect("fixture path should be utf8"),
        ])
        .output()
        .expect("compile command should execute");

    assert!(
        compile.status.success(),
        "expected llvm compile success for {}, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
        source.display(),
        compile.status.code(),
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let compile_stdout = String::from_utf8_lossy(&compile.stdout);
    let artifact = compile_stdout
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("compile: ok ").map(str::trim))
        .expect("compile stdout should include artifact path");

    let executable_path = if Path::new(artifact).is_absolute() {
        PathBuf::from(artifact)
    } else {
        temp_dir.join(artifact)
    };

    Command::new(&executable_path)
        .current_dir(temp_dir)
        .output()
        .expect("compiled executable should run")
}
