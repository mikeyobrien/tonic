use super::{parse_ast, Expr};
use crate::lexer::scan_tokens;

#[test]
fn parse_ast_reports_deterministic_map_entry_diagnostics() {
    let tokens = scan_tokens("defmodule Demo do\n  def run() do\n    %{1 2}\n  end\nend\n")
        .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject malformed map entries");

    assert!(
        error
            .to_string()
            .contains("expected map fat arrow `=>`, found INT(2)"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_supports_pin_patterns_case_guards_and_match_operator() {
    let tokens = scan_tokens(
        "defmodule PatternDemo do\n  def run() do\n    case list(7, 8) do\n      [^value, tail] when tail == 8 -> value = tail\n      _ -> 0\n    end\n  end\n\n  def value() do\n    7\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"case",
            "subject":{"kind":"call","callee":"list","args":[{"kind":"int","value":7},{"kind":"int","value":8}]},
            "branches":[
                {
                    "pattern":{
                        "kind":"list",
                        "items":[
                            {"kind":"pin","name":"value"},
                            {"kind":"bind","name":"tail"}
                        ]
                    },
                    "guard":{
                        "kind":"binary",
                        "op":"eq",
                        "left":{"kind":"variable","name":"tail"},
                        "right":{"kind":"int","value":8}
                    },
                    "body":{
                        "kind":"binary",
                        "op":"match",
                        "left":{"kind":"variable","name":"value"},
                        "right":{"kind":"variable","name":"tail"}
                    }
                },
                {
                    "pattern":{"kind":"wildcard"},
                    "body":{"kind":"int","value":0}
                }
            ]
        })
    );
}

#[test]
fn parse_ast_exposes_normalized_case_branch_head_and_body() {
    let tokens = scan_tokens(
        "defmodule PatternDemo do\n  def run() do\n    case input() do\n      {:ok, value} -> 1\n      _ -> 2\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");
    let Expr::Case { branches, .. } = &ast.modules[0].functions[0].body else {
        panic!("expected case expression body");
    };

    assert_eq!(branches.len(), 2);
    assert_eq!(
        serde_json::to_value(branches[0].head()).expect("branch head should serialize"),
        serde_json::json!({
            "kind":"tuple",
            "items":[
                {"kind":"atom","value":"ok"},
                {"kind":"bind","name":"value"}
            ]
        })
    );
    assert_eq!(
        serde_json::to_value(branches[0].body()).expect("branch body should serialize"),
        serde_json::json!({"kind":"int","value":1})
    );
}

#[test]
fn parse_ast_supports_function_head_patterns_defaults_and_private_defs() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def classify({:ok, value}) do\n    value\n  end\n\n  defp add(value, inc \\\\ 2) do\n    value + inc\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(ast.modules[0].functions[0].params[0].name(), "__arg0");
    assert_eq!(
        serde_json::to_value(ast.modules[0].functions[0].params[0].pattern())
            .expect("pattern should serialize"),
        serde_json::json!({
            "kind":"tuple",
            "items":[
                {"kind":"atom","value":"ok"},
                {"kind":"bind","name":"value"}
            ]
        })
    );
    assert!(ast.modules[0].functions[1].is_private());
    assert!(ast.modules[0].functions[1].params[1].default().is_some());
}

#[test]
fn parse_ast_supports_anonymous_functions_capture_and_invocation() {
    let tokens =
        scan_tokens("defmodule Demo do\n  def run() do\n    (&(&1 + 1)).(2)\n  end\nend\n")
            .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"invoke",
            "callee":{
                "kind":"group",
                "inner":{
                    "kind":"fn",
                    "params":["__capture1"],
                    "body":{
                        "kind":"binary",
                        "op":"plus",
                        "left":{"kind":"variable","name":"__capture1"},
                        "right":{"kind":"int","value":1}
                    }
                }
            },
            "args":[{"kind":"int","value":2}]
        })
    );
}

#[test]
fn parse_ast_supports_named_function_capture_shorthand() {
    let tokens = scan_tokens(
        "defmodule Math do\n  def add(left, right) do\n    left + right\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    (&Math.add/2).(1, 2)\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[1].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"invoke",
            "callee":{
                "kind":"group",
                "inner":{
                    "kind":"fn",
                    "params":["__capture1", "__capture2"],
                    "body":{
                        "kind":"call",
                        "callee":"Math.add",
                        "args":[
                            {"kind":"variable","name":"__capture1"},
                            {"kind":"variable","name":"__capture2"}
                        ]
                    }
                }
            },
            "args":[{"kind":"int","value":1}, {"kind":"int","value":2}]
        })
    );
}

#[test]
fn parse_ast_supports_multi_clause_anonymous_functions_with_guards() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    (fn {:ok, value} when is_integer(value) -> value; {:ok, _} -> -1; _ -> 0 end).({:ok, 4})\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    let Expr::Invoke { callee, .. } = &ast.modules[0].functions[0].body else {
        panic!("expected invoke expression body");
    };

    let Expr::Group { inner, .. } = callee.as_ref() else {
        panic!("expected grouped anonymous function callee");
    };

    let Expr::Fn { params, body, .. } = inner.as_ref() else {
        panic!("expected anonymous function callee");
    };

    assert_eq!(params, &vec!["__arg0".to_string()]);

    let Expr::Case { branches, .. } = body.as_ref() else {
        panic!("expected lowered case dispatch for anonymous function clauses");
    };

    assert_eq!(branches.len(), 3);
    assert!(branches[0].guard().is_some());
    assert!(branches[1].guard().is_none());
    assert!(branches[2].guard().is_none());
}

#[test]
fn parse_ast_supports_if_unless_cond_and_with_forms() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def pick(flag) do\n    if flag do\n      1\n    else\n      0\n    end\n  end\n\n  def reject(flag) do\n    unless flag do\n      2\n    else\n      3\n    end\n  end\n\n  def route(value) do\n    cond do\n      value > 2 -> 4\n      true -> 5\n    end\n  end\n\n  def chain() do\n    with [left, right] <- list(1, 2),\n         total <- left + right do\n      total\n    else\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");
    let functions = &ast.modules[0].functions;

    assert!(matches!(functions[0].body, Expr::Case { .. }));
    assert!(matches!(functions[1].body, Expr::Case { .. }));
    assert!(matches!(functions[2].body, Expr::Case { .. }));
    assert!(matches!(functions[3].body, Expr::Case { .. }));
}

#[test]
fn parse_ast_supports_for_comprehensions() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2, 3) do\n      x + 1\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"for",
            "into": null,
            "reduce": null,
            "generators":[
                {
                    "pattern":{"kind":"bind","name":"x"},
                    "source":{
                        "kind":"call",
                        "callee":"list",
                        "args":[
                            {"kind":"int","value":1},
                            {"kind":"int","value":2},
                            {"kind":"int","value":3}
                        ]
                    }
                }
            ],
            "body":{
                "kind":"binary",
                "op":"plus",
                "left":{"kind":"variable","name":"x"},
                "right":{"kind":"int","value":1}
            }
        })
    );
}

#[test]
fn parse_ast_supports_for_with_multiple_generators() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), y <- list(3, 4) do\n      x + y\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should not reject multi-generator for forms");

    let body_json = serde_json::to_value(&ast.modules[0].functions[0].body).unwrap();
    assert_eq!(body_json["kind"], "for");
    assert_eq!(body_json["generators"].as_array().unwrap().len(), 2);
}

#[test]
fn parse_ast_supports_for_reduce_and_generator_guards() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    for x when x > 1 <- list(1, 2), reduce: 0 do\n      acc -> acc + x\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should support reduce/guard for forms");
    let body_json = serde_json::to_value(&ast.modules[0].functions[0].body)
        .expect("expression should serialize");

    assert_eq!(body_json["kind"], "for");
    assert_eq!(body_json["reduce"]["kind"], "int");
    assert_eq!(body_json["reduce"]["value"], 0);
    assert_eq!(body_json["generators"][0]["guard"]["kind"], "binary");
    assert_eq!(body_json["body"]["kind"], "case");
}

#[test]
fn parse_ast_rejects_unsupported_for_options() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), uniq: true do\n      x\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject unsupported for options");

    assert_eq!(
        error.to_string(),
        "unsupported for option 'uniq'; supported options are into and reduce at offset 58"
    );
}

#[test]
fn parse_ast_rejects_non_trailing_default_params() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def add(value \\\\ 1, other) do\n    value + other\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject non-trailing default params");

    assert!(
        error
            .to_string()
            .starts_with("default parameters must be trailing at offset"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_if_end() {
    let tokens = scan_tokens("defmodule Demo do\n  def run(flag) do\n    if flag do\n      1\n")
        .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing if end");
    let message = error.to_string();

    assert!(
        message
            .starts_with("[E0003] unexpected end of file: missing 'end' to close if expression."),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("hint: add 'end' to finish if expression"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_if_do() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run(flag) do\n    if flag\n      1\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing if do");
    let message = error.to_string();

    assert!(
        message.starts_with("[E0006] missing 'do' to start if expression; found INT(1) instead."),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("hint: add 'do' after the if condition to begin the then branch"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_case_do() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run(value) do\n    case value\n      1 -> :one\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing case do");
    let message = error.to_string();

    assert!(
        message.starts_with("[E0006] missing 'do' to start case expression; found INT(1) instead."),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("hint: add 'do' after the case subject to begin the case branches"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_try_do() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    try\n      risky()\n    rescue\n      _ -> :error\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing try do");
    let message = error.to_string();

    assert!(
        message.starts_with(
            "[E0006] missing 'do' to start try expression; found IDENT(risky) instead."
        ),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("hint: add 'do' after 'try' to begin the protected block"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_arrow_in_case_style_clauses() {
    let cases = [
        (
            "case branch",
            "defmodule Demo do\n  def run(value) do\n    case value do\n      :ok value\n    end\n  end\nend\n",
            "[E0007] missing '->' in case branch; found IDENT(value) instead.",
            "add '->' after the case pattern before the branch body",
        ),
        (
            "with else clause",
            "defmodule Demo do\n  def run(result) do\n    with value <- result do\n      value\n    else\n      :error 0\n    end\n  end\nend\n",
            "[E0007] missing '->' in with else clause; found INT(0) instead.",
            "add '->' after the with else pattern before the fallback body",
        ),
        (
            "for reduce clause",
            "defmodule Demo do\n  def run() do\n    for x <- list(1, 2), reduce: 0 do\n      acc acc + x\n    end\n  end\nend\n",
            "[E0007] missing '->' in for reduce clause; found IDENT(acc) instead.",
            "add '->' after the accumulator pattern before the reduce body",
        ),
        (
            "try catch clause",
            "defmodule Demo do\n  def run() do\n    try do\n      risky()\n    catch\n      :throw :fallback\n    end\n  end\nend\n",
            "[E0007] missing '->' in try catch clause; found ATOM(fallback) instead.",
            "add '->' after the catch pattern before the clause body",
        ),
    ];

    for (clause, source, prefix, hint) in cases {
        let tokens = scan_tokens(source).expect("scanner should tokenize parser fixture");
        let error = parse_ast(&tokens).expect_err("parser should reject missing clause arrow");
        let message = error.to_string();

        assert!(
            message.starts_with(prefix),
            "unexpected parser error for {clause}: {error}"
        );
        assert!(
            message.contains(hint),
            "unexpected parser error for {clause}: {error}"
        );
    }
}

#[test]
fn parse_ast_reports_missing_arrow_in_cond_branch() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run(value) do\n    cond do\n      value > 2 4\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing cond arrow");
    let message = error.to_string();

    assert!(
        message.starts_with("[E0007] missing '->' in cond branch; found INT(4) instead."),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("hint: add '->' after the cond condition before the branch body"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_arrow_in_try_rescue_clause() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    try do\n      risky()\n    rescue\n      Demo.Error :error\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing rescue arrow");
    let message = error.to_string();

    assert!(
        message
            .starts_with("[E0007] missing '->' in try rescue clause; found ATOM(error) instead."),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("hint: add '->' after the rescue pattern before the clause body"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_arrow_in_anonymous_function_clause() {
    let tokens =
        scan_tokens("defmodule Demo do\n  def run() do\n    fn value value + 1 end\n  end\nend\n")
            .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing anonymous fn arrow");
    let message = error.to_string();

    assert!(
        message.starts_with(
            "[E0007] missing '->' in anonymous function clause; found IDENT(value) instead."
        ),
        "unexpected parser error: {error}"
    );
    assert!(
        message
            .contains("hint: add '->' between the anonymous function parameters and clause body"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_unexpected_arrow_outside_branch() {
    let tokens =
        scan_tokens("defmodule Demo do\n  def run() do\n    value -> value + 1\n  end\nend\n")
            .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject bare arrow outside branch");
    let message = error.to_string();

    assert!(
        message.starts_with("[E0004] unexpected '->' outside a valid branch."),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("hint: use 'fn ... -> ... end' for anonymous functions"),
        "unexpected parser error: {error}"
    );
    assert!(
        message.contains("case/cond/with/for/try"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_stray_clause_keywords_outside_valid_blocks() {
    let cases = [
        (
            "else",
            "defmodule Demo do\n  def run() do\n    else\n  end\nend\n",
            "[E0005] unexpected 'else' without a matching block.",
            "move 'else' inside an 'if', 'unless', or 'with' expression",
        ),
        (
            "rescue",
            "defmodule Demo do\n  def run() do\n    rescue\n  end\nend\n",
            "[E0005] unexpected 'rescue' without a matching 'try'.",
            "move 'rescue' inside a 'try ... end' expression",
        ),
        (
            "catch",
            "defmodule Demo do\n  def run() do\n    catch\n  end\nend\n",
            "[E0005] unexpected 'catch' without a matching 'try'.",
            "move 'catch' inside a 'try ... end' expression",
        ),
        (
            "after",
            "defmodule Demo do\n  def run() do\n    after\n  end\nend\n",
            "[E0005] unexpected 'after' without a matching 'try'.",
            "move 'after' inside a 'try ... end' expression",
        ),
    ];

    for (keyword, source, prefix, hint) in cases {
        let tokens = scan_tokens(source).expect("scanner should tokenize parser fixture");
        let error = parse_ast(&tokens).expect_err("parser should reject stray block keyword");
        let message = error.to_string();

        assert!(
            message.starts_with(prefix),
            "unexpected parser error for {keyword}: {error}"
        );
        assert!(
            message.contains(hint),
            "unexpected parser error for {keyword}: {error}"
        );
    }
}

#[test]
fn parse_ast_reports_stray_block_boundary_keywords() {
    let cases = [
        (
            "end",
            "defmodule Demo do\n  def run() do\n    end\n  end\nend\n",
            "[E0005] unexpected 'end' without an opening block.",
            "remove the extra 'end'",
        ),
        (
            "do",
            "defmodule Demo do\n  def run() do\n    do\n  end\nend\n",
            "[E0005] unexpected 'do' without a block header.",
            "put 'do' after a block opener like 'def', 'if', 'case', 'cond', 'with', 'for', or 'try'",
        ),
    ];

    for (keyword, source, prefix, hint) in cases {
        let tokens = scan_tokens(source).expect("scanner should tokenize parser fixture");
        let error = parse_ast(&tokens).expect_err("parser should reject stray block keyword");
        let message = error.to_string();

        assert!(
            message.starts_with(prefix),
            "unexpected parser error for {keyword}: {error}"
        );
        assert!(
            message.contains(hint),
            "unexpected parser error for {keyword}: {error}"
        );
    }
}
