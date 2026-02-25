use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;

mod common;

#[test]
fn compiled_runtime_supports_system_stdlib_path_exists() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-stdlib");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.path_exists(\".\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success()
        .stdout(contains("compile: ok"));

    let executable = fixture_root.join(".tonic/build/main");
    assert!(
        executable.exists(),
        "expected compiled executable at {}",
        executable.display()
    );

    let output = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");

    assert!(
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "true\n");
}

#[test]
fn compiled_runtime_system_argv_matches_interpreter_command_scan() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-argv");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    find(System.argv())\n  end\n\n  def find(argv) do\n    case argv do\n      [\"alpha\" | _] -> \"alpha\"\n      [_ | rest] -> find(rest)\n      [] -> \"none\"\n      _ -> \"none\"\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let interp = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", ".", "alpha"])
        .output()
        .expect("interpreter run should execute");

    assert!(
        interp.status.success(),
        "expected interpreter success, got status {:?} and stderr: {}",
        interp.status.code(),
        String::from_utf8_lossy(&interp.stderr)
    );
    assert_eq!(
        String::from_utf8(interp.stdout).expect("interpreter stdout should be utf8"),
        "\"alpha\"\n"
    );

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success()
        .stdout(contains("compile: ok"));

    let executable = fixture_root.join(".tonic/build/main");
    assert!(
        executable.exists(),
        "expected compiled executable at {}",
        executable.display()
    );

    let compiled = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .arg("alpha")
        .output()
        .expect("compiled executable should run");

    assert!(
        compiled.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        compiled.status.code(),
        String::from_utf8_lossy(&compiled.stderr)
    );

    assert_eq!(
        String::from_utf8(compiled.stdout).expect("compiled stdout should be utf8"),
        "\"alpha\"\n"
    );
}
