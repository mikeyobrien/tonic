use std::fs;
use std::path::Path;
mod common;

#[test]
fn check_reports_unclosed_grouped_expression_parse_error() {
    let stderr = run_check(
        "check-unclosed-grouped-expression",
        "unclosed_group.tn",
        "defmodule Demo do\n  def run() do\n    (1 + 2\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: grouped expression is missing ')'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add ')' to close the grouped expression"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_call_argument_list_parse_error() {
    let stderr = run_check(
        "check-unclosed-call-args",
        "unclosed_call.tn",
        "defmodule Demo do\n  def run() do\n    Math.add(1, 2\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: call argument list is missing ')'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`Math.add(left, right)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_capture_expression_parse_error() {
    let stderr = run_check(
        "check-unclosed-capture-expression",
        "unclosed_capture.tn",
        "defmodule Demo do\n  def run() do\n    &(&1 + 1\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: capture expression is missing ')'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`&(&1 + 1)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_index_access_parse_error() {
    let stderr = run_check(
        "check-unclosed-index-access",
        "unclosed_index.tn",
        "defmodule Demo do\n  def run(value) do\n    value[0\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: index access is missing ']'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`value[index]`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_function_param_list_parse_error() {
    let stderr = run_check(
        "check-unclosed-function-params",
        "unclosed_function_params.tn",
        "defmodule Demo do\n  def run(left, right do\n    left + right\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: function parameter list is missing ')'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`def run(left, right) do ... end`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_alias_child_list_parse_error() {
    let stderr = run_check(
        "check-unclosed-alias-child-list",
        "unclosed_alias_child_list.tn",
        "defmodule Demo do\n  alias Math.{Add, Sub\n\n  def run() do\n    Add.value()\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: alias child list is missing '}'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add '}' to close the alias child list"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`alias Math.{Bar, Baz}`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_import_filter_list_parse_error() {
    let stderr = run_check(
        "check-unclosed-import-filter-list",
        "unclosed_import_filter_list.tn",
        "defmodule Demo do\n  import Enum, only: [map: 2\n\n  def run() do\n    map([1], fn value -> value end)\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: import only filter list is missing ']'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add ']' to close the import only filter list"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`import Enum, only: [map: 2]`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_structured_raise_arguments_parse_error() {
    let stderr = run_check(
        "check-unclosed-structured-raise-arguments",
        "unclosed_structured_raise_arguments.tn",
        "defmodule Demo do\n  def run() do\n    raise(RuntimeError, message: \"oops\"\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: structured raise arguments is missing ')'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add ')' to close the structured raise arguments"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`raise(RuntimeError, message: \"oops\")`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_keyword_list_parse_error() {
    let stderr = run_check(
        "check-unclosed-keyword-list",
        "unclosed_keyword_list.tn",
        "defmodule Demo do\n  def run() do\n    [message: \"oops\", detail: 1\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: keyword list is missing ']'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add ']' to close the keyword list"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`[message: \"oops\", detail: info]`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_map_literal_parse_error() {
    let stderr = run_check(
        "check-unclosed-map-literal",
        "unclosed_map_literal.tn",
        "defmodule Demo do\n  def run() do\n    %{foo: 1, bar: 2\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: map literal is missing '}'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add '}' to close the map literal"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`%{foo: 1, bar: 2}`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_list_pattern_parse_error() {
    let stderr = run_check(
        "check-unclosed-list-pattern",
        "unclosed_list_pattern.tn",
        "defmodule Demo do\n  def run(value) do\n    case value do\n      [head, tail -> head\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: list pattern is missing ']'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add ']' to close the list pattern"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`[head, tail] -> ...`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_struct_pattern_parse_error() {
    let stderr = run_check(
        "check-unclosed-struct-pattern",
        "unclosed_struct_pattern.tn",
        "defmodule Demo do\n  def run(value) do\n    case value do\n      %User{name: name, age: age -> name\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: struct pattern is missing '}'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add '}' to close the struct pattern"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`%User{name: name, age: age} -> ...`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_bitstring_literal_parse_error() {
    let stderr = run_check(
        "check-unclosed-bitstring-literal",
        "unclosed_bitstring_literal.tn",
        "defmodule Demo do\n  def run() do\n    <<1, 2\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: bitstring literal is missing '>>'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add '>>' to close the bitstring literal"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`<<left, right>>`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_bitstring_pattern_parse_error() {
    let stderr = run_check(
        "check-unclosed-bitstring-pattern",
        "unclosed_bitstring_pattern.tn",
        "defmodule Demo do\n  def run(value) do\n    case value do\n      <<left, right -> left\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: bitstring pattern is missing '>>'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: add '>>' to close the bitstring pattern"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`<<left, right>> -> ...`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_unclosed_protocol_param_list_parse_error() {
    let stderr = run_check(
        "check-unclosed-protocol-params",
        "unclosed_protocol_params.tn",
        "defmodule Demo do\n  defprotocol Size do\n    def size(left, right\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0002] unclosed delimiter: protocol parameter list is missing ')'."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`def size(left, right)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

fn run_check(fixture_name: &str, file_name: &str, source: &str) -> String {
    let fixture_root = common::unique_fixture_root(fixture_name);
    let examples_dir = fixture_root.join("examples");

    fs::create_dir_all(&examples_dir).expect("fixture setup should create examples directory");
    let source_path = examples_dir.join(file_name);
    fs::write(&source_path, source).expect("fixture setup should write invalid source file");

    let relative_path = relative_example_path(&source_path);
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_tonic"))
        .current_dir(&fixture_root)
        .args(["check", relative_path.as_str()])
        .output()
        .expect("check command should run");

    assert!(
        !output.status.success(),
        "expected check command to fail for unclosed delimiters"
    );

    String::from_utf8(output.stderr).expect("stderr should be utf8")
}

fn relative_example_path(path: &Path) -> String {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .expect("fixture file name should be utf8");
    format!("examples/{file_name}")
}
