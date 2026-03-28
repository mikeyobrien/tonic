mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn create_project_fixture(
    test_name: &str,
    entry_source: &str,
    sample_text: Option<&str>,
) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(src_dir.join("main.tn"), entry_source)
        .expect("fixture setup should write entry source");
    if let Some(sample_text) = sample_text {
        fs::write(fixture_root.join("sample.txt"), sample_text)
            .expect("fixture setup should write sample text");
    }

    fixture_root
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

fn assert_compiled_stdout(
    test_name: &str,
    entry_source: &str,
    sample_text: Option<&str>,
    expected_stdout: &str,
) {
    let fixture_root = create_project_fixture(test_name, entry_source, sample_text);
    compile_fixture(&fixture_root);

    let output = run_compiled_fixture(&fixture_root);
    assert!(
        output.status.success(),
        "expected compiled executable success, got status {:?} and stderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("compiled stdout should be utf8");
    assert_eq!(stdout, expected_stdout);
}

#[test]
fn compiled_runtime_supports_string_stdlib_frontmatter_helper_set_on_literals() {
    let cases = [
        (
            "split",
            "defmodule Demo do\n  def run() do\n    String.split(\"a,b\", \",\")\n  end\nend\n",
            "[\"a\", \"b\"]\n",
        ),
        (
            "trim",
            "defmodule Demo do\n  def run() do\n    String.trim(\"  hello  \")\n  end\nend\n",
            "\"hello\"\n",
        ),
        (
            "replace",
            "defmodule Demo do\n  def run() do\n    String.replace(\"hello world world\", \"world\", \"Tonic\")\n  end\nend\n",
            "\"hello Tonic Tonic\"\n",
        ),
        (
            "trim-leading",
            "defmodule Demo do\n  def run() do\n    String.trim_leading(\"  hello  \")\n  end\nend\n",
            "\"hello  \"\n",
        ),
        (
            "trim-trailing",
            "defmodule Demo do\n  def run() do\n    String.trim_trailing(\"  hello  \")\n  end\nend\n",
            "\"  hello\"\n",
        ),
        (
            "starts-with",
            "defmodule Demo do\n  def run() do\n    String.starts_with(\"hello\", \"he\")\n  end\nend\n",
            "true\n",
        ),
        (
            "ends-with",
            "defmodule Demo do\n  def run() do\n    String.ends_with(\"hello\", \"lo\")\n  end\nend\n",
            "true\n",
        ),
        (
            "contains",
            "defmodule Demo do\n  def run() do\n    String.contains(\"hello world\", \"lo w\")\n  end\nend\n",
            "true\n",
        ),
        (
            "upcase",
            "defmodule Demo do\n  def run() do\n    String.upcase(\"hello\")\n  end\nend\n",
            "\"HELLO\"\n",
        ),
        (
            "upcase-unicode",
            "defmodule Demo do\n  def run() do\n    String.upcase(\"héß\")\n  end\nend\n",
            "\"HÉSS\"\n",
        ),
        (
            "downcase",
            "defmodule Demo do\n  def run() do\n    String.downcase(\"HELLO\")\n  end\nend\n",
            "\"hello\"\n",
        ),
        (
            "downcase-unicode",
            "defmodule Demo do\n  def run() do\n    String.downcase(\"İÉ\")\n  end\nend\n",
            "\"i̇é\"\n",
        ),
        (
            "length-unicode",
            "defmodule Demo do\n  def run() do\n    String.length(\"hé🙂\")\n  end\nend\n",
            "3\n",
        ),
        (
            "at-unicode",
            "defmodule Demo do\n  def run() do\n    String.at(\"hé🙂\", 1)\n  end\nend\n",
            "\"é\"\n",
        ),
        (
            "at-negative",
            "defmodule Demo do\n  def run() do\n    String.at(\"hello\", -1)\n  end\nend\n",
            "\"o\"\n",
        ),
        (
            "at-out-of-bounds",
            "defmodule Demo do\n  def run() do\n    String.at(\"hello\", 99)\n  end\nend\n",
            "nil\n",
        ),
        (
            "capitalize",
            "defmodule Demo do\n  def run() do\n    String.capitalize(\"tONIC\")\n  end\nend\n",
            "\"Tonic\"\n",
        ),
        (
            "capitalize-unicode-rest",
            "defmodule Demo do\n  def run() do\n    String.capitalize(\"hÉß\")\n  end\nend\n",
            "\"Héß\"\n",
        ),
        (
            "to-charlist-ascii",
            "defmodule Demo do\n  def run() do\n    String.to_charlist(\"tonic\")\n  end\nend\n",
            "[116, 111, 110, 105, 99]\n",
        ),
        (
            "to-charlist-unicode",
            "defmodule Demo do\n  def run() do\n    String.to_charlist(\"hé🙂\")\n  end\nend\n",
            "[104, 233, 128578]\n",
        ),
        (
            "slice",
            "defmodule Demo do\n  def run() do\n    String.slice(\"hello\", 1, 3)\n  end\nend\n",
            "\"ell\"\n",
        ),
        (
            "slice-unicode",
            "defmodule Demo do\n  def run() do\n    String.slice(\"hé🙂\", 1, 2)\n  end\nend\n",
            "\"é🙂\"\n",
        ),
        (
            "to-integer",
            "defmodule Demo do\n  def run() do\n    String.to_integer(\"7\")\n  end\nend\n",
            "7\n",
        ),
        (
            "to-float",
            "defmodule Demo do\n  def run() do\n    String.to_float(\" 7.5 \")\n  end\nend\n",
            "7.5\n",
        ),
        (
            "pad-leading-unicode",
            "defmodule Demo do\n  def run() do\n    String.pad_leading(\"é\", 4, \"🙂x\")\n  end\nend\n",
            "\"🙂x🙂é\"\n",
        ),
        (
            "pad-trailing-unicode",
            "defmodule Demo do\n  def run() do\n    String.pad_trailing(\"é\", 4, \"🙂x\")\n  end\nend\n",
            "\"é🙂x🙂\"\n",
        ),
        (
            "reverse-unicode",
            "defmodule Demo do\n  def run() do\n    String.reverse(\"hé🙂\")\n  end\nend\n",
            "\"🙂éh\"\n",
        ),
        (
            "to-atom",
            "defmodule Demo do\n  def run() do\n    String.to_atom(\"tonic\")\n  end\nend\n",
            ":tonic\n",
        ),
        (
            "graphemes-unicode",
            "defmodule Demo do\n  def run() do\n    String.graphemes(\"hé🙂\")\n  end\nend\n",
            "[\"h\", \"é\", \"🙂\"]\n",
        ),
    ];

    for (suffix, entry_source, expected_stdout) in cases {
        let test_name = format!("runtime-llvm-string-stdlib-{suffix}");
        assert_compiled_stdout(&test_name, entry_source, None, expected_stdout);
    }
}

#[test]
fn compiled_runtime_supports_string_stdlib_on_system_read_text_content() {
    let cases = [
        (
            "split-file",
            "defmodule Demo do\n  def run() do\n    String.split(System.read_text(\"sample.txt\"), \",\")\n  end\nend\n",
            "[\"  first\", \"second  \"]\n",
        ),
        (
            "trim-file",
            "defmodule Demo do\n  def run() do\n    String.trim(System.read_text(\"sample.txt\"))\n  end\nend\n",
            "\"first,second\"\n",
        ),
    ];

    for (suffix, entry_source, expected_stdout) in cases {
        let test_name = format!("runtime-llvm-string-stdlib-{suffix}");
        assert_compiled_stdout(
            &test_name,
            entry_source,
            Some("  first,second  "),
            expected_stdout,
        );
    }
}

#[test]
fn compiled_runtime_string_to_integer_reports_parse_failure_deterministically() {
    let fixture_root = create_project_fixture(
        "runtime-llvm-string-stdlib-to-integer-failure",
        "defmodule Demo do\n  def run() do\n    String.to_integer(\"abc\")\n  end\nend\n",
        None,
    );
    compile_fixture(&fixture_root);

    let output = run_compiled_fixture(&fixture_root);
    assert!(
        !output.status.success(),
        "expected compiled executable failure for invalid integer"
    );

    let stderr = String::from_utf8(output.stderr).expect("compiled stderr should be utf8");
    assert!(
        stderr.contains("error: host error: String.to_integer could not parse \"abc\" as integer"),
        "expected deterministic parse-failure message, got: {stderr}"
    );
}

#[test]
fn compiled_runtime_string_to_float_reports_parse_failure_deterministically() {
    let fixture_root = create_project_fixture(
        "runtime-llvm-string-stdlib-to-float-failure",
        "defmodule Demo do\n  def run() do\n    String.to_float(\"abc\")\n  end\nend\n",
        None,
    );
    compile_fixture(&fixture_root);

    let output = run_compiled_fixture(&fixture_root);
    assert!(
        !output.status.success(),
        "expected compiled executable failure for invalid float"
    );

    let stderr = String::from_utf8(output.stderr).expect("compiled stderr should be utf8");
    assert!(
        stderr.contains("error: host error: String.to_float could not parse \"abc\" as float"),
        "expected deterministic parse-failure message, got: {stderr}"
    );
}
