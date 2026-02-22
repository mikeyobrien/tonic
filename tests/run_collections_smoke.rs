use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_tuple_map_keyword_constructors_and_prints_rendered_value() {
    let fixture_root = unique_fixture_root("run-collections-smoke");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_collections.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(map(1, 2), keyword(3, 4))\n  end\nend\n",
    )
    .expect("fixture setup should write collections source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_collections.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "{%{1 => 2}, [3: 4]}\n");
}

#[test]
fn run_executes_collection_literals_and_matches_constructor_rendering() {
    let fixture_root = unique_fixture_root("run-collection-literals");
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    fs::write(
        examples_dir.join("run_collection_literals.tn"),
        "defmodule Demo do\n  def run() do\n    tuple({1, 2}, tuple([3, 4], tuple(%{ok: 5}, [done: 6])))\n  end\nend\n",
    )
    .expect("fixture setup should write collections source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/run_collection_literals.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "{{1, 2}, {[3, 4], {%{:ok => 5}, [done: 6]}}}\n");
}

fn unique_fixture_root(test_name: &str) -> PathBuf {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!(
        "tonic-{test_name}-{timestamp}-{}",
        std::process::id()
    ))
}
