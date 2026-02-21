use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_primitive_literals_and_prints_rendered_value() {
    let fixture_root = unique_fixture_root("run-primitives");
    let src_dir = fixture_root.join("examples");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");

    fs::write(
        src_dir.join("primitives.tn"),
        "defmodule Demo do\n  def run() do\n    tuple(true, tuple(false, tuple(nil, \"hello world\")))\n  end\nend\n",
    )
    .expect("fixture setup should write source file");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "examples/primitives.tn"])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "{true, {false, {nil, \"hello world\"}}}\n");
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
