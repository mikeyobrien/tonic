use assert_cmd::assert::OutputAssertExt;
use predicates::str::contains;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Stdio;

mod common;

fn write_project_fixture(fixture_root: &Path, source: &str) {
    let src_dir = fixture_root.join("src");
    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(src_dir.join("main.tn"), source).expect("fixture setup should write entry source");
}

fn compile_fixture(fixture_root: &Path) -> PathBuf {
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
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
    executable
}

#[test]
fn compiled_runtime_supports_io_ansi_helpers_and_inspect_return_semantics() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-io-ansi-inspect");
    write_project_fixture(
        &fixture_root,
        "defmodule Demo do\n  def run() do\n    case IO.inspect(%{ok: 1}) do\n      inspected ->\n        {inspected, {IO.ansi_red(\"red\"), {IO.ansi_green(\"green\"), {IO.ansi_yellow(\"yellow\"), {IO.ansi_blue(\"blue\"), IO.ansi_reset()}}}}}\n    end\n  end\nend\n",
    );

    let executable = compile_fixture(&fixture_root);
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
        "{%{:ok => 1}, {\"\u{1b}[31mred\u{1b}[0m\", {\"\u{1b}[32mgreen\u{1b}[0m\", {\"\u{1b}[33myellow\u{1b}[0m\", {\"\u{1b}[34mblue\u{1b}[0m\", \"\u{1b}[0m\"}}}}}\n"
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
        "%{:ok => 1}\n"
    );
}

#[test]
fn compiled_runtime_suppresses_final_value_after_io_puts_stdout() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-io-puts");
    write_project_fixture(
        &fixture_root,
        "defmodule Demo do\n  def run() do\n    case IO.puts(\"hello from puts\") do\n      _ -> :done\n    end\n  end\nend\n",
    );

    let executable = compile_fixture(&fixture_root);
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
        "hello from puts\n"
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
        ""
    );
}

#[test]
fn compiled_runtime_suppresses_final_value_after_io_gets_prompt() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-io-gets");
    write_project_fixture(
        &fixture_root,
        "defmodule Demo do\n  def run() do\n    IO.gets(\"prompt> \")\n  end\nend\n",
    );

    let executable = compile_fixture(&fixture_root);
    let mut child = std::process::Command::new(&executable)
        .current_dir(&fixture_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("compiled executable should spawn");

    let mut stdin = child.stdin.take().expect("stdin pipe should be available");
    stdin
        .write_all(b"typed line\n")
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

    assert_eq!(
        String::from_utf8(output.stdout).expect("stdout should be utf8"),
        "prompt> "
    );
    assert_eq!(
        String::from_utf8(output.stderr).expect("stderr should be utf8"),
        ""
    );
}
