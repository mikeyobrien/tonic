use std::fs;
mod common;

#[test]
fn run_suppresses_final_value_when_app_prints_to_stdout() {
    let fixture_root = common::unique_fixture_root("run-command-stdout-contract-puts");
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

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
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
    assert_eq!(stdout, "hi\n");
}

#[test]
fn run_still_prints_final_value_when_app_does_not_write_stdout() {
    let fixture_root = common::unique_fixture_root("run-command-stdout-contract-value");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    7\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
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
    assert_eq!(stdout, "7\n");
}
