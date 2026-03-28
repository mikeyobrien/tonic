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
fn compiled_runtime_supports_system_list_files_recursive_for_spaced_nested_directory() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-list-files-recursive");
    let src_dir = fixture_root.join("src");
    let assets_dir = fixture_root.join("assets with space").join("docs");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&assets_dir).expect("fixture setup should create nested asset directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"assets with space\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(
        fixture_root.join("assets with space").join("style.css"),
        "root",
    )
    .expect("fixture setup should write root asset");
    fs::write(assets_dir.join("guide.css"), "nested")
        .expect("fixture setup should write nested asset");

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
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("\"docs/guide.css\"") && stdout.contains("\"style.css\""),
        "expected nested + root file paths in output, got: {stdout}"
    );
    assert!(
        stdout.find("docs/guide.css").unwrap_or(usize::MAX)
            < stdout.find("style.css").unwrap_or(usize::MAX),
        "expected deterministic sorted order, got: {stdout}"
    );
}

#[test]
fn compiled_runtime_supports_system_remove_tree_for_spaced_nested_directory() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-remove-tree");
    let src_dir = fixture_root.join("src");
    let output_dir = fixture_root.join("out with space").join("docs");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&output_dir).expect("fixture setup should create nested output directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    {System.remove_tree(\"out with space\"), System.remove_tree(\"out with space\")}\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(output_dir.join("guide.css"), "nested")
        .expect("fixture setup should write nested output file");

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
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "{true, false}\n"
    );
    assert!(
        !fixture_root.join("out with space").exists(),
        "expected remove_tree target to be gone"
    );
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
fn compiled_runtime_supports_system_append_text() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-append-text");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.append_text(\"payload.txt\", \"alpha\\n\")\n    System.append_text(\"payload.txt\", \"beta\\n\")\n    System.read_text(\"payload.txt\")\n  end\nend\n",
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
    assert_eq!(stdout, "\"alpha\nbeta\n\"\n");
    assert_eq!(
        fs::read_to_string(fixture_root.join("payload.txt"))
            .expect("payload.txt should exist after append_text"),
        "alpha\nbeta\n"
    );
}

#[test]
fn compiled_runtime_supports_system_write_text_atomic_and_lock_primitives() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-persistence-locks");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.write_text_atomic(\"state/payload.txt\", \"snapshot-v1\")\n    [\n      System.write_text_atomic(\"state/payload.txt\", \"snapshot-v2\"),\n      System.read_text(\"state/payload.txt\"),\n      System.lock_acquire(\"locks/proposal.lock\"),\n      System.lock_acquire(\"locks/proposal.lock\"),\n      System.lock_release(\"locks/proposal.lock\"),\n      System.lock_release(\"locks/proposal.lock\")\n    ]\n  end\nend\n",
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
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "[true, \"snapshot-v2\", true, false, true, false]\n"
    );
    assert_eq!(
        fs::read_to_string(fixture_root.join("state").join("payload.txt"))
            .expect("payload.txt should exist after atomic write"),
        "snapshot-v2"
    );
    assert!(
        !fixture_root.join("locks").join("proposal.lock").exists(),
        "expected lock file to be absent after release"
    );
}

#[test]
fn compiled_runtime_lock_acquire_writes_observable_marker_with_positive_timestamp() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-lock-marker");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    [System.lock_acquire(\"locks/proposal.lock\"), System.read_text(\"locks/proposal.lock\")]\n  end\nend\n",
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
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.starts_with("[true, \"pid=") && stdout.contains(" timestamp_ms="),
        "expected observable lock marker in compiled output, got: {stdout}"
    );

    let marker = fs::read_to_string(fixture_root.join("locks").join("proposal.lock"))
        .expect("lock marker should exist after acquire");
    assert!(
        marker.starts_with("pid=") && marker.contains(" timestamp_ms="),
        "expected lock marker shape, got: {marker}"
    );

    let timestamp_text = marker
        .split("timestamp_ms=")
        .nth(1)
        .expect("marker should include timestamp_ms")
        .trim();
    let timestamp_ms: i64 = timestamp_text
        .parse()
        .expect("timestamp_ms should be a positive integer");
    assert!(timestamp_ms > 0, "expected timestamp_ms > 0, got: {timestamp_ms}");
}

#[test]
fn compiled_runtime_system_write_text_atomic_rejects_non_string_content_deterministically() {
    let fixture_root =
        common::unique_fixture_root("runtime-llvm-system-write-text-atomic-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    host_call(:sys_write_text_atomic, \"payload.txt\", true)\n  end\nend\n",
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
        stderr.contains(
            "error: host error: sys_write_text_atomic expects string argument 2; found bool"
        ),
        "expected deterministic type-error message, got: {stderr}"
    );
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

#[test]
fn compiled_runtime_list_files_recursive_skips_symlinked_entries() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-list-files-recursive-symlink");
    let src_dir = fixture_root.join("src");
    let real_dir = fixture_root.join("tree").join("real");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&real_dir).expect("fixture setup should create real sub-directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"tree\")\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");
    fs::write(fixture_root.join("tree").join("root.txt"), "root")
        .expect("fixture setup should write root file");
    fs::write(real_dir.join("nested.txt"), "nested")
        .expect("fixture setup should write nested file");

    let link_file = fixture_root.join("tree").join("linkfile.txt");
    let link_dir = fixture_root.join("tree").join("linkdir");
    std::os::unix::fs::symlink(fixture_root.join("tree").join("root.txt"), &link_file)
        .expect("fixture setup should create symlink to file");
    std::os::unix::fs::symlink(&real_dir, &link_dir)
        .expect("fixture setup should create symlink to directory");

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
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(
        stdout.contains("\"real/nested.txt\"") && stdout.contains("\"root.txt\""),
        "expected only real file paths in output, got: {stdout}"
    );
    assert!(
        !stdout.contains("linkfile.txt"),
        "expected symlinked file to be excluded from native output, got: {stdout}"
    );
    assert!(
        !stdout.contains("linkdir"),
        "expected symlinked directory contents to be excluded from native output, got: {stdout}"
    );
}

#[test]
fn compiled_runtime_list_files_recursive_errors_on_missing_path() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-list-files-recursive-missing");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"no_such_dir\")\n  end\nend\n",
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
        "expected compiled executable failure for missing path"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_list_files_recursive failed for 'no_such_dir'"),
        "expected deterministic missing-path error from native, got: {stderr}"
    );
}

#[test]
fn compiled_runtime_list_files_recursive_rejects_non_string_argument() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-list-files-recursive-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(42)\n  end\nend\n",
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
        stderr.contains(
            "error: host error: sys_list_files_recursive expects string argument 1; found int"
        ),
        "expected deterministic type-error message from native, got: {stderr}"
    );
}

#[test]
fn compiled_runtime_list_files_recursive_rejects_empty_path() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-list-files-recursive-empty-path");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.list_files_recursive(\"\")\n  end\nend\n",
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
        "expected compiled executable failure for empty path"
    );
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(
        stderr.contains("error: host error: sys_list_files_recursive path must not be empty"),
        "expected deterministic empty-path error from native, got: {stderr}"
    );
}

#[test]
fn compiled_runtime_remove_tree_removes_symlinked_file_as_file() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-remove-tree-symlink-file");
    let src_dir = fixture_root.join("src");
    let target_file = fixture_root.join("real.txt");
    let link_file = fixture_root.join("linkfile.txt");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(&target_file, "real content").expect("fixture setup should write real file");
    std::os::unix::fs::symlink(&target_file, &link_file)
        .expect("fixture setup should create symlink to file");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.remove_tree(\"linkfile.txt\")\n  end\nend\n",
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
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "true\n",
        "expected true when symlink was removed"
    );
    assert!(
        link_file.symlink_metadata().is_err(),
        "expected symlink to be removed by native runtime"
    );
    assert!(target_file.exists(), "expected real file to survive");
}

#[test]
fn compiled_runtime_remove_tree_on_symlinked_directory_removes_symlink_only() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-remove-tree-symlink-dir");
    let src_dir = fixture_root.join("src");
    let real_dir = fixture_root.join("realdir");
    let link_dir = fixture_root.join("linkdir");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::create_dir_all(&real_dir).expect("fixture setup should create real directory");
    fs::write(real_dir.join("inside.txt"), "content")
        .expect("fixture setup should write file inside real directory");
    std::os::unix::fs::symlink(&real_dir, &link_dir)
        .expect("fixture setup should create symlink to directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.remove_tree(\"linkdir\")\n  end\nend\n",
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
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "true\n",
        "expected true when symlink-to-directory was removed"
    );
    assert!(
        link_dir.symlink_metadata().is_err(),
        "expected symlink to directory to be removed by native runtime"
    );
    assert!(
        real_dir.exists(),
        "expected real directory to survive after symlink removal"
    );
    assert!(
        real_dir.join("inside.txt").exists(),
        "expected real directory contents to survive"
    );
}

#[test]
fn compiled_runtime_remove_tree_rejects_non_string_argument() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-remove-tree-type-error");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    System.remove_tree(42)\n  end\nend\n",
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
        stderr.contains("error: host error: sys_remove_tree expects string argument 1; found int"),
        "expected deterministic type-error message from native, got: {stderr}"
    );
}
