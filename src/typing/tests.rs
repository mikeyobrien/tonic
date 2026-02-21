use super::diag::TypingDiagnosticCode;
use super::infer_types;
use crate::lexer::scan_tokens;
use crate::parser::parse_ast;

#[test]
fn infer_types_supports_polymorphic_like_helper_with_concrete_call_sites() {
    let source = "defmodule Demo do\n  def helper(value) do\n    1\n  end\n\n  def one() do\n    1\n  end\n\n  def run() do\n    helper(1) + helper(one())\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize typing fixture");
    let ast = parse_ast(&tokens).expect("parser should build typing fixture ast");

    let summary = infer_types(&ast)
        .expect("type inference should succeed for helper reuse across call sites");

    assert_eq!(summary.signature("Demo.helper"), Some("fn(dynamic) -> int"));
    assert_eq!(summary.signature("Demo.run"), Some("fn() -> int"));
}

#[test]
fn infer_types_reports_type_mismatch_with_span_offset() {
    let source = "defmodule Demo do\n  def unknown() do\n    ok(1)\n  end\n\n  def run() do\n    unknown() + 1\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize mismatch fixture");
    let ast = parse_ast(&tokens).expect("parser should build mismatch fixture ast");

    let error = infer_types(&ast).expect_err("type inference should reject non-int operands for +");

    assert_eq!(
        error.to_string(),
        "[E2001] type mismatch: expected int, found result at offset 73"
    );
}

#[test]
fn infer_types_supports_question_operator_for_result_values() {
    let source = "defmodule Demo do\n  def run() do\n    ok(1)?\n  end\nend\n";
    let tokens =
        scan_tokens(source).expect("scanner should tokenize question-operator typing fixture");
    let ast = parse_ast(&tokens).expect("parser should build question-operator typing fixture ast");

    let summary =
        infer_types(&ast).expect("type inference should accept ? when operand is a Result value");

    assert_eq!(summary.signature("Demo.run"), Some("fn() -> int"));
}

#[test]
fn infer_types_accepts_collection_constructor_builtins() {
    let source =
        "defmodule Demo do\n  def run() do\n    tuple(map(1, 2), keyword(3, 4))\n  end\nend\n";
    let tokens =
        scan_tokens(source).expect("scanner should tokenize collection constructor fixture");
    let ast = parse_ast(&tokens).expect("parser should build collection constructor fixture ast");

    let summary = infer_types(&ast)
        .expect("type inference should accept tuple/map/keyword constructor builtins");

    assert_eq!(summary.signature("Demo.run"), Some("fn() -> dynamic"));
}

#[test]
fn infer_types_accepts_protocol_dispatch_builtin_calls() {
    let source = "defmodule Demo do\n  def run() do\n    tuple(protocol_dispatch(tuple(1, 2)), protocol_dispatch(map(3, 4)))\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize protocol dispatch fixture");
    let ast = parse_ast(&tokens).expect("parser should build protocol dispatch fixture ast");

    let summary =
        infer_types(&ast).expect("type inference should accept protocol dispatch builtin calls");

    assert_eq!(summary.signature("Demo.run"), Some("fn() -> dynamic"));
}

#[test]
fn infer_types_threads_pipe_input_into_enum_style_calls() {
    let source = "defmodule Enum do\n  def stage_one(_value) do\n    1\n  end\n\n  def stage_two(_value) do\n    2\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    tuple(1, 2) |> Enum.stage_one() |> Enum.stage_two()\n  end\nend\n";
    let tokens = scan_tokens(source).expect("scanner should tokenize pipe fixture");
    let ast = parse_ast(&tokens).expect("parser should build pipe fixture ast");

    let summary =
        infer_types(&ast).expect("type inference should treat pipe rhs call as receiving lhs");

    assert_eq!(
        summary.signature("Enum.stage_one"),
        Some("fn(dynamic) -> int")
    );
    assert_eq!(
        summary.signature("Enum.stage_two"),
        Some("fn(dynamic) -> int")
    );
    assert_eq!(summary.signature("Demo.run"), Some("fn() -> int"));
}

#[test]
fn infer_types_accepts_explicit_dynamic_parameter_annotation() {
    let source = "defmodule Demo do\n  def helper(dynamic value) do\n    1\n  end\n\n  def run() do\n    helper(1)\n  end\nend\n";
    let tokens =
        scan_tokens(source).expect("scanner should tokenize explicit dynamic parameter fixture");
    let ast =
        parse_ast(&tokens).expect("parser should accept explicit dynamic parameter annotations");

    let summary = infer_types(&ast)
        .expect("type inference should accept explicit dynamic parameter annotations");

    assert_eq!(summary.signature("Demo.helper"), Some("fn(dynamic) -> int"));
    assert_eq!(summary.signature("Demo.run"), Some("fn() -> int"));
}

#[test]
fn parse_ast_rejects_dynamic_annotation_outside_parameter_positions() {
    let source = "defmodule Demo do\n  def run() -> dynamic do\n    1\n  end\nend\n";
    let tokens =
        scan_tokens(source).expect("scanner should tokenize invalid dynamic annotation fixture");

    let error = parse_ast(&tokens)
        .expect_err("parser should reject dynamic annotations outside parameter positions");

    assert_eq!(
        error.to_string(),
        "dynamic annotation is only allowed on parameters at offset 30"
    );
}

#[test]
fn infer_types_reports_non_exhaustive_case_without_wildcard_branch() {
    let source = "defmodule Demo do\n  def run() do\n    case value() do\n      :ok -> 1\n    end\n  end\n\n  def value() do\n    1\n  end\nend\n";
    let tokens =
        scan_tokens(source).expect("scanner should tokenize non-exhaustive case typing fixture");
    let ast = parse_ast(&tokens).expect("parser should build non-exhaustive case typing fixture");

    let error = infer_types(&ast)
        .expect_err("type inference should reject non-exhaustive case expressions");

    assert_eq!(
        error.to_string(),
        "[E3002] non-exhaustive case expression: missing wildcard branch at offset 37"
    );
}

#[test]
fn infer_types_harmonizes_result_and_match_diagnostics() {
    let question_source =
        "defmodule Demo do\n  def value() do\n    1\n  end\n\n  def run() do\n    value()?\n  end\nend\n";
    let question_tokens =
        scan_tokens(question_source).expect("scanner should tokenize question mismatch fixture");
    let question_ast =
        parse_ast(&question_tokens).expect("parser should build question mismatch fixture ast");

    let question_error =
        infer_types(&question_ast).expect_err("type inference should reject question on int");

    assert_eq!(
        question_error.code(),
        Some(TypingDiagnosticCode::QuestionRequiresResult)
    );
    assert_eq!(
        question_error.message(),
        "? operator requires Result value, found int"
    );

    let case_source = "defmodule Demo do\n  def run() do\n    case value() do\n      :ok -> 1\n    end\n  end\n\n  def value() do\n    1\n  end\nend\n";
    let case_tokens =
        scan_tokens(case_source).expect("scanner should tokenize non-exhaustive case fixture");
    let case_ast =
        parse_ast(&case_tokens).expect("parser should build non-exhaustive case fixture ast");

    let case_error =
        infer_types(&case_ast).expect_err("type inference should reject non-exhaustive case");

    assert_eq!(
        case_error.code(),
        Some(TypingDiagnosticCode::NonExhaustiveCase)
    );
    assert_eq!(
        case_error.message(),
        "non-exhaustive case expression: missing wildcard branch"
    );
}
