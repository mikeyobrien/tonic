use std::fs;
use std::path::{Path, PathBuf};
mod common;

#[test]
fn json_decode_object() {
    let fixture_root = create_project_fixture(
        "json-decode-object",
        r#"defmodule Demo do
  def run() do
    result = Json.decode("{\"name\":\"alice\",\"age\":30}")
    result["name"]
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "\"alice\"\n");
}

#[test]
fn json_decode_array() {
    let fixture_root = create_project_fixture(
        "json-decode-array",
        r#"defmodule Demo do
  def run() do
    Json.decode("[1,2,3]")
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "[1, 2, 3]\n");
}

#[test]
fn json_encode_map() {
    let fixture_root = create_project_fixture(
        "json-encode-map",
        r#"defmodule Demo do
  def run() do
    Json.encode(%{"key" => 42})
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "\"{\"key\":42}\"\n");
}

#[test]
fn json_extract_field_returns_value() {
    let fixture_root = create_project_fixture(
        "json-extract-field",
        r#"defmodule Demo do
  def run() do
    Json.extract_field("{\"name\":\"bob\",\"age\":25}", "name")
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "\"bob\"\n");
}

#[test]
fn json_extract_field_returns_nil_for_missing() {
    let fixture_root = create_project_fixture(
        "json-extract-field-nil",
        r#"defmodule Demo do
  def run() do
    Json.extract_field("{\"name\":\"bob\"}", "missing")
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "nil\n");
}

#[test]
fn json_extract_path_nested() {
    let fixture_root = create_project_fixture(
        "json-extract-path",
        r#"defmodule Demo do
  def run() do
    json = "{\"assistantMessageEvent\":{\"delta\":\"hello world\"}}"
    Json.extract_path(json, "assistantMessageEvent.delta")
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "\"hello world\"\n");
}

#[test]
fn json_extract_path_deeply_nested() {
    let fixture_root = create_project_fixture(
        "json-extract-path-deep",
        r#"defmodule Demo do
  def run() do
    json = "{\"a\":{\"b\":{\"c\":42}}}"
    Json.extract_path(json, "a.b.c")
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "42\n");
}

#[test]
fn json_extract_path_missing_returns_nil() {
    let fixture_root = create_project_fixture(
        "json-extract-path-missing",
        r#"defmodule Demo do
  def run() do
    Json.extract_path("{\"a\":{\"b\":1}}", "a.c")
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "nil\n");
}

#[test]
fn json_parse_object_ok() {
    let fixture_root = create_project_fixture(
        "json-parse-object-ok",
        r#"defmodule Demo do
  def run() do
    case Json.parse_object("{\"x\":1}") do
      {:ok, map} -> map["x"]
      {:error, reason} -> reason
      _ -> "unexpected"
    end
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "1\n");
}

#[test]
fn json_parse_object_rejects_array() {
    let fixture_root = create_project_fixture(
        "json-parse-object-reject",
        r#"defmodule Demo do
  def run() do
    case Json.parse_object("[1,2]") do
      {:ok, _} -> "unexpected ok"
      {:error, reason} -> reason
      _ -> "unexpected"
    end
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "\"expected JSON object, got different type\"\n");
}

#[test]
fn json_parse_array_ok() {
    let fixture_root = create_project_fixture(
        "json-parse-array-ok",
        r#"defmodule Demo do
  def run() do
    case Json.parse_array("[10,20,30]") do
      {:ok, list} -> list
      {:error, reason} -> reason
      _ -> "unexpected"
    end
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "[10, 20, 30]\n");
}

#[test]
fn json_stream_parse_accumulates() {
    let fixture_root = create_project_fixture(
        "json-stream-parse",
        r#"defmodule Demo do
  def run() do
    lines = [
      "{\"value\":1}",
      "{\"value\":2}",
      "{\"value\":3}"
    ]
    Json.stream_parse(lines, 0, fn parsed, acc -> acc + parsed["value"] end)
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "6\n");
}

#[test]
fn json_stream_parse_skips_malformed_lines() {
    let fixture_root = create_project_fixture(
        "json-stream-parse-malformed",
        r#"defmodule Demo do
  def run() do
    lines = [
      "{\"value\":10}",
      "not json",
      "{\"value\":20}"
    ]
    Json.stream_parse(lines, 0, fn parsed, acc -> acc + parsed["value"] end)
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "30\n");
}

#[test]
fn json_roundtrip_encode_decode() {
    let fixture_root = create_project_fixture(
        "json-roundtrip",
        r#"defmodule Demo do
  def run() do
    original = %{"name" => "test", "items" => [1, 2, 3]}
    encoded = Json.encode(original)
    decoded = Json.decode(encoded)
    decoded["items"]
  end
end
"#,
    );

    let output = run_project(&fixture_root);
    assert_success(&output);
    assert_eq!(stdout(&output), "[1, 2, 3]\n");
}

fn create_project_fixture(test_name: &str, entry_source: &str) -> PathBuf {
    let fixture_root = common::unique_fixture_root(test_name);
    let src_dir = fixture_root.join("src");

    fs::create_dir_all(&src_dir).expect("fixture setup should create src directory");
    fs::write(
        fixture_root.join("tonic.toml"),
        "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
    )
    .expect("fixture setup should write tonic.toml");
    fs::write(src_dir.join("main.tn"), entry_source)
        .expect("fixture setup should write entry module source");

    fixture_root
}

fn run_project(fixture_root: &Path) -> std::process::Output {
    std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(fixture_root)
        .args(["run", "."])
        .output()
        .expect("run command should execute")
}

fn stdout(output: &std::process::Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("stdout should be utf8")
}

fn assert_success(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "expected success, got status {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
