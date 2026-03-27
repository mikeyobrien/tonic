mod common;

use std::fs;
use std::process::Command;

#[test]
fn compiled_runtime_retains_output_across_repeated_system_run_calls() {
    let fixture_root = common::unique_fixture_root("runtime-llvm-system-run-repeat");
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(
        src_dir.join("main.tn"),
        "defmodule Demo do\n  def run() do\n    \"first: \" <> summarize(System.run(\"printf '%s' fresh-run-one\")) <> \"\\nsecond: \" <> summarize(System.run(\"printf '%s' fresh-run-two\"))\n  end\n\n  defp summarize(result) do\n    case result do\n      %{exit_code: code, output: output} ->\n        \"exit+output exit_code=#{code} output=#{output}\"\n\n      %{output: output} ->\n        \"output-only output=#{output}\"\n\n      %{exit_code: code} ->\n        \"exit-only exit_code=#{code}\"\n\n      _ ->\n        \"unexpected\"\n    end\n  end\nend\n",
    )
    .expect("fixture setup should write entry source");

    let interpreted = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["run", "."])
        .output()
        .expect("interpreter run should execute");

    assert!(
        interpreted.status.success(),
        "expected interpreter success, got status {:?} and stderr: {}",
        interpreted.status.code(),
        String::from_utf8_lossy(&interpreted.stderr)
    );
    assert_eq!(
        String::from_utf8(interpreted.stdout).expect("interpreter stdout should be utf8"),
        "\"first: exit+output exit_code=0 output=fresh-run-one\nsecond: exit+output exit_code=0 output=fresh-run-two\"\n"
    );

    let compile = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["compile", "."])
        .output()
        .expect("compile command should execute");

    assert!(
        compile.status.success(),
        "expected compile success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        compile.status.code(),
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let compiled = Command::new(fixture_root.join(".tonic/build/main"))
        .current_dir(&fixture_root)
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
        "\"first: exit+output exit_code=0 output=fresh-run-one\nsecond: exit+output exit_code=0 output=fresh-run-two\"\n"
    );
}
