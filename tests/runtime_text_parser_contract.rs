mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const ENTRY_SOURCE: &str = "defmodule Demo do
  def run() do
    {
      binary_status(runtime_text()),
      {
        list_status(runtime_text()),
        {
          list_prefix_match(runtime_text()),
          {
            bitstring_prefix_match(runtime_text()),
            {
              control_list_prefix_match(),
              {
                control_bitstring_prefix_match(),
                String.starts_with(runtime_text(), \"+++\")
              }
            }
          }
        }
      }
    }
  end

  defp runtime_text() do
    System.read_text(\"sample.txt\")
  end

  defp binary_status(value) when is_binary(value) do
    true
  end

  defp binary_status(_value) do
    false
  end

  defp list_status(value) when is_list(value) do
    true
  end

  defp list_status(_value) do
    false
  end

  defp list_prefix_match(value) do
    case value do
      [43, 43, 43, 10 | _rest] -> true
      _ -> false
    end
  end

  defp bitstring_prefix_match(value) do
    case value do
      <<a, b, c, d>> -> true
      _ -> false
    end
  end

  defp control_list_prefix_match() do
    case [43, 43, 43, 10, 35] do
      [43, 43, 43, 10 | _rest] -> true
      _ -> false
    end
  end

  defp control_bitstring_prefix_match() do
    case <<43, 43, 43, 10>> do
      <<a, b, c, d>> -> true
      _ -> false
    end
  end
end
";
const SAMPLE_TEXT: &str = "+++\ntitle = \"Fixture\"\n+++\n\n# Fixture\n";
const EXPECTED_STDOUT: &str = "{true, {false, {false, {false, {true, {true, true}}}}}}\n";

fn create_project_fixture(test_name: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(src_dir.join("main.tn"), ENTRY_SOURCE)
        .expect("fixture setup should write entry source");
    fs::write(fixture_root.join("sample.txt"), SAMPLE_TEXT)
        .expect("fixture setup should write sample text");

    fixture_root
}

fn run_project_fixture(fixture_root: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute")
}

fn compile_fixture(fixture_root: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .args(["compile", "."])
        .output()
        .expect("compile command should execute");

    assert!(
        output.status.success(),
        "expected compile success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("compile stdout should be utf8");
    assert!(
        stdout.contains("compile: ok"),
        "expected compile success marker, got stdout:\n{stdout}"
    );
}

fn run_compiled_fixture(fixture_root: &Path) -> Output {
    Command::new(fixture_root.join(".tonic/build/main"))
        .current_dir(fixture_root)
        .output()
        .expect("compiled executable should run")
}

fn assert_success_stdout(output: Output, expected_stdout: &str, context: &str) {
    assert!(
        output.status.success(),
        "expected {context} success, got status {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert_eq!(stdout, expected_stdout, "stdout mismatch for {context}");
}

#[test]
fn project_runtime_text_is_binary_not_list_and_not_parser_ready_for_byte_patterns() {
    let fixture_root = create_project_fixture("runtime-text-parser-contract");

    assert_success_stdout(
        run_project_fixture(&fixture_root),
        EXPECTED_STDOUT,
        "interpreter run",
    );

    compile_fixture(&fixture_root);
    assert_success_stdout(
        run_compiled_fixture(&fixture_root),
        EXPECTED_STDOUT,
        "compiled runtime",
    );
}
