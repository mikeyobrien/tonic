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
    assert!(
        !stderr.contains("module-load stdlib:System"),
        "expected optional stdlib System module to stay unloaded when unreferenced, got: {stderr:?}"
    );
}

#[test]
fn run_trace_does_not_lazy_load_enum_after_deadvertising() {
    let fixture_root = create_project_fixture(
        "run-lazy-enum-stdlib-deadvertised",
        "defmodule Demo do\n  def run() do\n    Enum.count([1, 2, 3])\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        !output.status.success(),
        "expected Enum fixture to fail after de-advertising, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load project:Demo"),
        "expected module-load trace to include project entry module, got: {stderr:?}"
    );
    assert!(
        !stderr.contains("module-load stdlib:Enum"),
        "expected Enum stdlib module to stay de-advertised, got: {stderr:?}"
    );
    assert!(
        stderr.contains("undefined symbol 'Enum.count'"),
        "expected undefined-symbol failure after removing Enum injection, got: {stderr:?}"
    );
}

#[test]
fn run_trace_lazy_loads_system_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-system-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    System.path_exists(\".\")\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for System-stdlib fixture, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "true\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load stdlib:System"),
        "expected module-load trace to include lazy-loaded System stdlib module, got: {stderr:?}"
    );
}

#[test]
fn run_trace_lazy_loads_string_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-string-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    String.split(\"a,b\", \",\")\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for String-stdlib fixture, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[\"a\", \"b\"]\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load stdlib:String"),
        "expected module-load trace to include lazy-loaded String stdlib module, got: {stderr:?}"
    );
}

#[test]
fn run_trace_lazy_loads_path_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-path-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    Path.join(\"assets\", \"index.html\")\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for Path-stdlib fixture, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "\"assets/index.html\"\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load stdlib:Path"),
        "expected module-load trace to include lazy-loaded Path stdlib module, got: {stderr:?}"
    );
}

#[test]
fn run_trace_does_not_lazy_load_list_after_deadvertising() {
    let fixture_root = create_project_fixture(
        "run-lazy-list-stdlib-deadvertised",
        "defmodule Demo do\n  def run() do\n    List.first([1, 2, 3])\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        !output.status.success(),
        "expected List fixture to fail after de-advertising, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("module-load stdlib:List"),
        "expected List stdlib module to stay de-advertised, got: {stderr:?}"
    );
    assert!(
        stderr.contains("undefined symbol 'List.first'"),
        "expected undefined-symbol failure after removing List injection, got: {stderr:?}"
    );
}

#[test]
fn run_trace_does_not_lazy_load_io_after_deadvertising() {
    let fixture_root = create_project_fixture(
        "run-lazy-io-stdlib-deadvertised",
        "defmodule Demo do\n  def run() do\n    IO.inspect(123)\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        !output.status.success(),
        "expected IO fixture to fail after de-advertising, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("module-load stdlib:IO"),
        "expected IO stdlib module to stay de-advertised, got: {stderr:?}"
    );
    assert!(
        stderr.contains("undefined symbol 'IO.inspect'"),
        "expected undefined-symbol failure after removing IO injection, got: {stderr:?}"
    );
}

#[test]
fn run_trace_does_not_lazy_load_map_after_deadvertising() {
    let fixture_root = create_project_fixture(
        "run-lazy-map-stdlib-deadvertised",
        "defmodule Demo do\n  def run() do\n    Map.keys(%{a: 1})\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        !output.status.success(),
        "expected Map fixture to fail after de-advertising, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        !stderr.contains("module-load stdlib:Map"),
        "expected Map stdlib module to stay de-advertised, got: {stderr:?}"
    );
    assert!(
        stderr.contains("undefined symbol 'Map.keys'"),
        "expected undefined-symbol failure after removing Map injection, got: {stderr:?}"
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
