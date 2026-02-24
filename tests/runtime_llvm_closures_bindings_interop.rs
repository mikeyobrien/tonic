mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[test]
fn compiled_llvm_runtime_matches_catalog_for_closure_binding_and_interop_fixtures() {
    let repo_root = std::env::current_dir().expect("repo root should be readable");
    let temp_dir = common::unique_temp_dir("runtime-llvm-closure-binding-interop");

    for (fixture, expected_stdout) in fixture_contracts() {
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
fn compiled_llvm_runtime_supports_protocol_dispatch_interop() {
    let temp_dir = common::unique_temp_dir("runtime-llvm-protocol-dispatch-interop");
    let source_path = temp_dir.join("protocol_dispatch_interop.tn");
    fs::write(
        &source_path,
        "defmodule Demo do\n  def run() do\n    tuple(protocol_dispatch(tuple(1, 2)), protocol_dispatch(map(3, 4)))\n  end\nend\n",
    )
    .expect("fixture should be written");

    let runtime = compile_and_run_fixture(&temp_dir, &source_path);

    assert!(
        runtime.status.success(),
        "expected runtime success, got exit {:?}\nstdout:\n{}\nstderr:\n{}",
        runtime.status.code(),
        String::from_utf8_lossy(&runtime.stdout),
        String::from_utf8_lossy(&runtime.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&runtime.stdout), "{1, 2}\n");
}

#[test]
fn compiled_llvm_runtime_reports_deterministic_interop_errors() {
    let temp_dir = common::unique_temp_dir("runtime-llvm-interop-errors");

    let unknown_host_path = temp_dir.join("unknown_host.tn");
    fs::write(
        &unknown_host_path,
        "defmodule Demo do\n  def run() do\n    host_call(:missing, 1)\n  end\nend\n",
    )
    .expect("unknown host fixture should be written");

    let unknown_host_runtime = compile_and_run_fixture(&temp_dir, &unknown_host_path);
    assert!(
        !unknown_host_runtime.status.success(),
        "expected unknown host fixture to fail"
    );
    assert!(
        String::from_utf8_lossy(&unknown_host_runtime.stderr)
            .contains("error: host error: unknown host function: missing"),
        "expected deterministic unknown-host error, got stderr:\n{}",
        String::from_utf8_lossy(&unknown_host_runtime.stderr)
    );

    let unsupported_protocol_path = temp_dir.join("unsupported_protocol.tn");
    fs::write(
        &unsupported_protocol_path,
        "defmodule Demo do\n  def run() do\n    protocol_dispatch(\"hello\")\n  end\nend\n",
    )
    .expect("unsupported protocol fixture should be written");

    let unsupported_protocol_runtime =
        compile_and_run_fixture(&temp_dir, &unsupported_protocol_path);
    assert!(
        !unsupported_protocol_runtime.status.success(),
        "expected unsupported protocol fixture to fail"
    );
    assert!(
        String::from_utf8_lossy(&unsupported_protocol_runtime.stderr)
            .contains("error: protocol_dispatch has no implementation for string"),
        "expected deterministic protocol-dispatch error, got stderr:\n{}",
        String::from_utf8_lossy(&unsupported_protocol_runtime.stderr)
    );
}

fn fixture_contracts() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "examples/parity/05-functions/anonymous_fn_capture_invoke.tn",
            "{10, 6}\n",
        ),
        ("examples/parity/06-control-flow/cond_branches.tn", "2\n"),
        (
            "examples/parity/08-errors/host_call_and_protocol_dispatch.tn",
            "\"hello interop\"\n",
        ),
    ]
}

fn compile_and_run_fixture(temp_dir: &Path, source: &Path) -> std::process::Output {
    let compile = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(temp_dir)
        .args([
            "compile",
            source.to_str().expect("fixture path should be utf8"),
            "--backend",
            "llvm",
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
