use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
use std::io::Write;
use std::process::Stdio;

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
fn compiled_runtime_supports_path_stdlib_join() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-path-stdlib");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    Path.join(\"assets\", \"index.html\")\n  end\nend\n",
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
    assert_eq!(stdout, "\"assets/index.html\"\n");
}

#[test]
fn compiled_runtime_supports_system_read_text() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-read-text");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_text(\"payload.txt\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(fixture_root.join("payload.txt"), "hello from file")
        .expect("fixture setup should write payload");

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
    assert_eq!(stdout, "\"hello from file\"\n");
}

#[test]
fn compiled_runtime_system_read_text_rejects_non_string_argument_deterministically() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-read-text-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_text(42)\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success()
        .stdout(contains("compile: ok"));

    let executable = fixture_root.join(".tonic/build/main");
    let output = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");

    assert!(
        !output.status.success(),
        "expected compiled executable failure for wrong argument type"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_read_text expects string argument 1; found int"),
        "expected deterministic type-error message, got: {stderr}"
    );
}

#[test]
fn compiled_runtime_system_read_text_reports_missing_file() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-read-text-missing");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_text(\"missing.txt\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success()
        .stdout(contains("compile: ok"));

    let executable = fixture_root.join(".tonic/build/main");
    let output = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");

    assert!(
        !output.status.success(),
        "expected compiled executable failure for missing file"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_read_text failed for 'missing.txt':"),
        "expected deterministic io-error prefix, got: {stderr}"
    );
}

#[test]
fn compiled_runtime_supports_system_read_stdin() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-read-stdin");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.read_stdin()\n  end\nend\n",
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

    let mut child = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled executable should spawn");

    let mut stdin = child.stdin.take().expect("stdin pipe should be available");
    stdin
        .write_all(b"compiled stdin")
        .expect("stdin write should succeed");
    drop(stdin);

    let output = child
        .wait_with_output()
        .expect("compiled executable should complete");

    assert!(
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, "\"compiled stdin\"\n");
}

#[test]
fn compiled_runtime_system_read_stdin_rejects_unexpected_arguments_deterministically() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-read-stdin-arity");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    host_call(:sys_read_stdin, \"unexpected\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .assert()
        .success()
        .stdout(contains("compile: ok"));

    let executable = fixture_root.join(".tonic/build/main");
    let output = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .output()
        .expect("compiled executable should run");

    assert!(
        !output.status.success(),
        "expected compiled executable failure for unexpected arguments"
    );

    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_read_stdin expects exactly 0 arguments, found 1"),
        "expected deterministic arity-error message, got: {stderr}"
    );
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
