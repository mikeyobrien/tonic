use std::fs;
use std::path::{Path, PathBuf};
mod common;

#[test]
fn run_trace_skips_optional_stdlib_modules_when_unreferenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-stdlib-unreferenced",
        "defmodule Demo do\n  def run() do\n    7\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for unreferenced stdlib fixture, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "7\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load project:Demo"),
        "expected module-load trace to include project entry module, got: {stderr:?}"
    );
    assert!(
        !stderr.contains("module-load stdlib:Enum"),
        "expected optional stdlib Enum module to stay unloaded when unreferenced, got: {stderr:?}"
    );
}

#[test]
fn run_trace_lazy_loads_optional_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    Enum.identity()\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for stdlib-referenced fixture, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "1\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load project:Demo"),
        "expected module-load trace to include project entry module, got: {stderr:?}"
    );
    assert!(
        stderr.contains("module-load stdlib:Enum"),
        "expected module-load trace to include lazy-loaded Enum stdlib module, got: {stderr:?}"
    );
}

fn create_project_fixture(test_name: &str, entry_source: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(src_dir.join("main.tn"), entry_source)
        .expect("fixture setup should write entry module source");

    fixture_root
}

fn run_with_module_trace(fixture_root: &Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .env("TONIC_DEBUG_MODULE_LOADS", "1")
        .args(["run", "."])
        .output()
        .expect("run command should execute")
}
