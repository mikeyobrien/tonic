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
    for module_name in ["Enum", "System", "IO", "List", "Map"] {
        assert!(
            !stderr.contains(&format!("module-load stdlib:{module_name}")),
            "expected optional stdlib {module_name} module to stay unloaded when unreferenced, got: {stderr:?}"
        );
    }
}

#[test]
fn run_trace_lazy_loads_enum_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-enum-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    {\n      {Enum.count([1, 2, 3]), Enum.sum(1..4)},\n      {\n        {Enum.reverse(1..4), {Enum.take(1..6, 2), Enum.drop(1..6, 2)}},\n        {\n          {Enum.chunk_every(1..6, 2), Enum.unique([1, 2, 1, 3, 2])},\n          {\n            {Enum.into(1..4, [0]), Enum.into([{:a, 1}, {:b, 2}], %{seed: 0})},\n            {Enum.join([\"a\", 1, :ok], \",\"), Enum.sort([3, 1, 2])}\n          }\n        }\n      }\n    }\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for Enum-stdlib fixture, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout,
        "{{3, 10}, {{[4, 3, 2, 1], {[1, 2], [3, 4, 5, 6]}}, {{[[1, 2], [3, 4], [5, 6]], [1, 2, 3]}, {{[0, 1, 2, 3, 4], %{:seed => 0, :a => 1, :b => 2}}, {\"a,1,:ok\", [1, 2, 3]}}}}}\n"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load project:Demo"),
        "expected module-load trace to include project entry module, got: {stderr:?}"
    );
    assert!(
        stderr.contains("module-load stdlib:Enum"),
        "expected Enum stdlib module to lazy-load, got: {stderr:?}"
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
fn run_trace_supports_string_to_charlist_in_project_mode() {
    let fixture_root = create_project_fixture(
        "run-lazy-string-to-charlist-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    String.to_charlist(\"hé\")\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for String.to_charlist fixture, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[104, 233]\n");

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
fn run_trace_lazy_loads_list_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-list-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    {List.first([1, 2, 3]), {List.last([1, 2, 3]), {List.wrap(nil), {List.flatten([1, [2, [3]], 4]), {List.zip([1, 2], [:a, :b, :c]), List.unzip([{1, :a}, {2, :b}])}}}}}\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for List-stdlib fixture, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(
        stdout,
        "{1, {3, {[], {[1, 2, 3, 4], {[{1, :a}, {2, :b}], {[1, 2], [:a, :b]}}}}}}\n"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load stdlib:List"),
        "expected List stdlib module to lazy-load, got: {stderr:?}"
    );
}

#[test]
fn run_trace_lazy_loads_io_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-io-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    IO.inspect(123)\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for IO-stdlib fixture, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "123\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load stdlib:IO"),
        "expected IO stdlib module to lazy-load, got: {stderr:?}"
    );
    assert!(
        stderr.contains("123"),
        "expected IO.inspect to emit inspected value on stderr, got: {stderr:?}"
    );
}

#[test]
fn run_trace_lazy_loads_map_stdlib_module_when_referenced() {
    let fixture_root = create_project_fixture(
        "run-lazy-map-stdlib-referenced",
        "defmodule Demo do\n  def run() do\n    Map.keys(%{a: 1})\n  end\nend\n",
    );

    let output = run_with_module_trace(&fixture_root);

    assert!(
        output.status.success(),
        "expected successful run for Map-stdlib fixture, got status {:?}, stdout: {}, stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "[:a]\n");

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("module-load stdlib:Map"),
        "expected Map stdlib module to lazy-load, got: {stderr:?}"
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
