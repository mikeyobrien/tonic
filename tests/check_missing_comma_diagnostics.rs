use std::fs;
use std::path::Path;
mod common;

#[test]
fn check_reports_parenthesized_call_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-call-paren",
        "call_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    Math.add(1 2)\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in call arguments; found INT(2) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("hint: separate call arguments with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`Math.add(left, right)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_no_paren_call_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-call-no-paren",
        "call_no_paren_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    tuple 1 2\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in call arguments; found INT(2) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`tuple(left, right)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_with_clause_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-with-clauses",
        "with_clause_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    with ok <- ok(1)\n         value <- ok + 1 do\n      value\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in with clauses; found IDENT(value) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate with clauses with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_for_generator_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-for-generators",
        "for_generator_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2)\n        y <- list(3, 4) do\n      {x, y}\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in for clauses; found IDENT(y) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate for generators and options with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_for_option_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-for-options",
        "for_option_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), into: []\n        reduce: 0 do\n      acc -> acc + x\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in for clauses; found IDENT(reduce) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate for generators and options with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_alias_child_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-alias-children",
        "alias_child_missing_comma.tn",
        "defmodule Demo do\n  alias Math.{Add Sub}\n\n  def run() do\n    Add.value()\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in alias child list; found IDENT(Sub) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate alias children with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`alias Math.{Bar, Baz}`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_import_filter_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-import-filter",
        "import_filter_missing_comma.tn",
        "defmodule Demo do\n  import Enum, only: [map: 2 reduce: 3]\n\n  def run() do\n    map([1], fn value -> value end)\n  end\nend\n",
    );

    assert!(
        stderr.contains(
            "[E0010] missing ',' in import only filter list; found IDENT(reduce) instead."
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate import only entries with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`import Enum, only: [map: 2, reduce: 3]`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_structured_raise_keyword_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-structured-raise",
        "structured_raise_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    raise(RuntimeError, message: \"oops\" detail: 1)\n  end\nend\n",
    );

    assert!(
        stderr.contains(
            "[E0010] missing ',' in structured raise arguments; found IDENT(detail) instead."
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate structured raise keyword arguments with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`raise(RuntimeError, message: \"oops\", detail: info)`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_list_literal_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-list-literal",
        "list_literal_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    [1 2]\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in list literal; found INT(2) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate list elements with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`[left, right]`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_keyword_list_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-keyword-list",
        "keyword_list_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    [message: \"oops\" detail: 1]\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in keyword list; found IDENT(detail) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate keyword entries with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`[message: \"oops\", detail: info]`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_map_literal_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-map-literal",
        "map_literal_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    %{foo: 1 bar: 2}\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in map literal; found IDENT(bar) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate map entries with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`%{foo: 1, bar: 2}`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_struct_literal_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-struct-literal",
        "struct_literal_missing_comma.tn",
        "defmodule Demo do\n  def run(user) do\n    %User{name: user age: user}\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in struct literal; found IDENT(age) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate struct fields with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`%User{name: name, age: age}`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_list_pattern_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-list-pattern",
        "list_pattern_missing_comma.tn",
        "defmodule Demo do\n  def run(value) do\n    case value do\n      [head tail] -> head\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in list pattern; found IDENT(tail) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate list pattern items with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`[head, tail]`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_struct_pattern_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-struct-pattern",
        "struct_pattern_missing_comma.tn",
        "defmodule Demo do\n  def run(value) do\n    case value do\n      %User{name: name age: age} -> name\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in struct pattern; found IDENT(age) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate struct pattern fields with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`%User{name: name, age: age}`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_bitstring_literal_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-bitstring-literal",
        "bitstring_literal_missing_comma.tn",
        "defmodule Demo do\n  def run() do\n    <<1 2>>\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in bitstring literal; found INT(2) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate bitstring elements with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`<<left, right>>`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_bitstring_pattern_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-bitstring-pattern",
        "bitstring_pattern_missing_comma.tn",
        "defmodule Demo do\n  def run(value) do\n    case value do\n      <<left right>> -> left\n    end\n  end\nend\n",
    );

    assert!(
        stderr.contains("[E0010] missing ',' in bitstring pattern; found IDENT(right) instead."),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("separate bitstring pattern elements with commas"),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`<<left, right>>`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_function_param_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-function-params",
        "function_params_missing_comma.tn",
        "defmodule Demo do\n  def run(left right) do\n    left + right\n  end\nend\n",
    );

    assert!(
        stderr.contains(
            "[E0010] missing ',' in function parameter list; found IDENT(right) instead."
        ),
        "unexpected parser diagnostic: {stderr}"
    );
    assert!(
        stderr.contains("`def run(left, right) do ... end`"),
        "unexpected parser diagnostic: {stderr}"
    );
}

#[test]
fn check_reports_protocol_param_missing_comma_parse_error() {
    let stderr = run_check(
        "check-missing-comma-protocol-params",
        "protocol_params_missing_comma.tn",
        "defmodule Demo do\n  defprotocol Size do\n    def size(left right)\n  end\nend\n",
    );

    assert!(
        stderr.contains(
            "[E0010] missing ',' in protocol parameter list; found IDENT(right) instead."
        ),
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
        "expected check command to fail for malformed comma syntax"
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
