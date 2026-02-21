use std::fs;
use std::path::PathBuf;

#[test]
fn run_executes_logical_operators_and_prints_rendered_value() {
    let fixture_root = unique_fixture_root("run-logical");
    let project_dir = fixture_root.join("logical_project");

    fs::create_dir_all(&project_dir).expect("fixture setup should create project directory");

    fs::write(
        project_dir.join("tonic.toml"),
        "[project]\nname = \"logical_project\"\nentry = \"main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        project_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    not false and (true or false) && 1 || 2\n  end\nend\n",
    )
    .expect("fixture setup should write main.tn");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_dir)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "1");
}

#[test]
fn run_executes_collection_operators_and_prints_rendered_value() {
    let fixture_root = unique_fixture_root("run-collection");
    let project_dir = fixture_root.join("collection_project");

    fs::create_dir_all(&project_dir).expect("fixture setup should create project directory");

    fs::write(
        project_dir.join("tonic.toml"),
        "[project]\nname = \"collection_project\"\nentry = \"main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");

    fs::write(
        project_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    \"hello\" <> \" \" <> \"world\"\n  end\nend\n",
    )
    .expect("fixture setup should write main.tn");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&project_dir)
        .args(["run", "."])
        .output()
        .expect("run command should execute");

    assert!(
        output.status.success(),
        "expected successful run invocation, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout.trim(), "\"hello world\"");
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