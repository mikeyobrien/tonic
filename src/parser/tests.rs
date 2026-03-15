use super::{parse_ast, Expr};
use crate::lexer::scan_tokens;

#[test]
fn parse_ast_supports_single_module_with_two_functions() {
    let tokens = scan_tokens(
        "defmodule Math do\n  def one() do\n    1\n  end\n\n  def two() do\n    one()\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(ast.modules.len(), 1);
    assert_eq!(ast.modules[0].name, "Math");
    assert_eq!(ast.modules[0].functions.len(), 2);
    assert_eq!(ast.modules[0].functions[0].name, "one");
    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({"kind":"int","value":1})
    );
    assert_eq!(ast.modules[0].functions[1].name, "two");
    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[1].body)
            .expect("expression should serialize"),
        serde_json::json!({"kind":"call","callee":"one","args":[]})
    );
}

#[test]
fn parse_ast_supports_nested_calls_with_plus_precedence() {
    let tokens = scan_tokens(
        "defmodule Math do\n  def compute() do\n    combine(1, 2) + wrap(inner(3 + 4))\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"binary",
            "op":"plus",
            "left":{
                "kind":"call",
                "callee":"combine",
                "args":[
                    {"kind":"int","value":1},
                    {"kind":"int","value":2}
                ]
            },
            "right":{
                "kind":"call",
                "callee":"wrap",
                "args":[
                    {
                        "kind":"call",
                        "callee":"inner",
                        "args":[
                            {
                                "kind":"binary",
                                "op":"plus",
                                "left":{"kind":"int","value":3},
                                "right":{"kind":"int","value":4}
                            }
                        ]
                    }
                ]
            }
        })
    );
}

#[test]
fn parse_ast_supports_module_qualified_calls() {
    let tokens = scan_tokens("defmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n")
        .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({"kind":"call","callee":"Math.helper","args":[]})
    );
}

#[test]
fn parse_ast_supports_no_paren_calls() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def helper(value) do\n    value\n  end\n\n  def run() do\n    helper 7\n  end\nend\n",
    )
    .expect("scanner should tokenize no-paren call fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[1].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"call",
            "callee":"helper",
            "args":[{"kind":"int","value":7}]
        })
    );
}

#[test]
fn parse_ast_supports_no_paren_module_qualified_calls() {
    let tokens = scan_tokens(
        "defmodule Math do\n  def one(value) do\n    value\n  end\nend\n\ndefmodule Demo do\n  def run() do\n    Math.one 7\n  end\nend\n",
    )
    .expect("scanner should tokenize no-paren qualified call fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[1].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"call",
            "callee":"Math.one",
            "args":[{"kind":"int","value":7}]
        })
    );
}

#[test]
fn parse_ast_supports_try_as_no_paren_call_arg() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def helper(value) do\n    value\n  end\n\n  def run() do\n    helper try do\n      :ok\n    rescue\n      _ -> :err\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize try no-paren call fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert!(
        matches!(ast.modules[0].functions[1].body, Expr::Call { .. }),
        "outermost expr should be call"
    );
}

#[test]
fn parse_ast_supports_raise_as_no_paren_call_arg() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def helper(value) do\n    value\n  end\n\n  def run() do\n    helper raise \"boom\"\n  end\nend\n",
    )
    .expect("scanner should tokenize raise no-paren call fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert!(
        matches!(ast.modules[0].functions[1].body, Expr::Call { .. }),
        "outermost expr should be call"
    );
}

#[test]
fn parse_ast_supports_postfix_question_operator() {
    let tokens = scan_tokens("defmodule Demo do\n  def run() do\n    value()?\n  end\nend\n")
        .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"question",
            "value":{"kind":"call","callee":"value","args":[]}
        })
    );
}

#[test]
#[ignore = "bitstring literals in expression position not yet implemented"]
fn parse_ast_supports_bitstring_literals_as_list_values() {
    let tokens = scan_tokens("defmodule Demo do\n  def run() do\n    <<1, 2, 3>>\n  end\nend\n")
        .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"list",
            "items":[
                {"kind":"int","value":1},
                {"kind":"int","value":2},
                {"kind":"int","value":3}
            ]
        })
    );
}

#[test]
fn parse_ast_supports_case_patterns() {
    let tokens = scan_tokens(
        "defmodule PatternDemo do\n  def run() do\n    case input() do\n      {:ok, value} -> 1\n      [head, tail] -> 2\n      %{} -> 3\n      _ -> 4\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"case",
            "subject":{"kind":"call","callee":"input","args":[]},
            "branches":[
                {
                    "pattern":{
                        "kind":"tuple",
                        "items":[
                            {"kind":"atom","value":"ok"},
                            {"kind":"bind","name":"value"}
                        ]
                    },
                    "body":{"kind":"int","value":1}
                },
                {
                    "pattern":{
                        "kind":"list",
                        "items":[
                            {"kind":"bind","name":"head"},
                            {"kind":"bind","name":"tail"}
                        ]
                    },
                    "body":{"kind":"int","value":2}
                },
                {
                    "pattern":{"kind":"map","entries":[]},
                    "body":{"kind":"int","value":3}
                },
                {
                    "pattern":{"kind":"wildcard"},
                    "body":{"kind":"int","value":4}
                }
            ]
        })
    );
}

#[test]
fn parse_ast_supports_list_cons_patterns() {
    let tokens = scan_tokens(
        "defmodule PatternDemo do\n  def run() do\n    case input() do\n      [head | tail] -> head\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"case",
            "subject":{"kind":"call","callee":"input","args":[]},
            "branches":[
                {
                    "pattern":{
                        "kind":"list",
                        "items":[{"kind":"bind","name":"head"}],
                        "tail":{"kind":"bind","name":"tail"}
                    },
                    "body":{"kind":"variable","name":"head"}
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
fn parse_ast_supports_map_colon_patterns() {
    let tokens = scan_tokens(
        "defmodule PatternDemo do\n  def run() do\n    case input() do\n      %{ok: value} -> value\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"case",
            "subject":{"kind":"call","callee":"input","args":[]},
            "branches":[
                {
                    "pattern":{
                        "kind":"map",
                        "entries":[
                            {
                                "key":{"kind":"atom","value":"ok"},
                                "value":{"kind":"bind","name":"value"}
                            }
                        ]
                    },
                    "body":{"kind":"variable","name":"value"}
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
fn parse_ast_supports_map_fat_arrow_literals_and_mixed_entries() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  def run() do\n    %{\"status\" => 200, ok: true, 1 => false}\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"map",
            "entries":[
                {
                    "key":{"kind":"string","value":"status"},
                    "value":{"kind":"int","value":200}
                },
                {
                    "key":{"kind":"atom","value":"ok"},
                    "value":{"kind":"bool","value":true}
                },
                {
                    "key":{"kind":"int","value":1},
                    "value":{"kind":"bool","value":false}
                }
            ]
        })
    );
}

#[test]
fn parse_ast_supports_map_fat_arrow_patterns() {
    let tokens = scan_tokens(
        "defmodule PatternDemo do\n  def run() do\n    case input() do\n      %{\"status\" => code, true => flag, ok: value} -> tuple(code, tuple(flag, value))\n      _ -> 0\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"case",
            "subject":{"kind":"call","callee":"input","args":[]},
            "branches":[
                {
                    "pattern":{
                        "kind":"map",
                        "entries":[
                            {
                                "key":{"kind":"string","value":"status"},
                                "value":{"kind":"bind","name":"code"}
                            },
                            {
                                "key":{"kind":"bool","value":true},
                                "value":{"kind":"bind","name":"flag"}
                            },
                            {
                                "key":{"kind":"atom","value":"ok"},
                                "value":{"kind":"bind","name":"value"}
                            }
                        ]
                    },
                    "body":{
                        "kind":"call",
                        "callee":"tuple",
                        "args":[
                            {"kind":"variable","name":"code"},
                            {
                                "kind":"call",
                                "callee":"tuple",
                                "args":[
                                    {"kind":"variable","name":"flag"},
                                    {"kind":"variable","name":"value"}
                                ]
                            }
                        ]
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
fn parse_ast_supports_defstruct_literals_and_updates() {
    let tokens = scan_tokens(
        "defmodule User do\n  defstruct name: \"\", age: 0\n\n  def run(user) do\n    {%User{name: \"A\"}, %User{user | age: 43}}\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].forms).expect("module forms should serialize"),
        serde_json::json!([
            {
                "kind":"defstruct",
                "fields":[
                    {"name":"name","default":{"kind":"string","value":""}},
                    {"name":"age","default":{"kind":"int","value":0}}
                ]
            }
        ])
    );

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"tuple",
            "items":[
                {
                    "kind":"struct",
                    "module":"User",
                    "entries":[
                        {"key":"name","value":{"kind":"string","value":"A"}}
                    ]
                },
                {
                    "kind":"structupdate",
                    "module":"User",
                    "base":{"kind":"variable","name":"user"},
                    "updates":[
                        {"key":"age","value":{"kind":"int","value":43}}
                    ]
                }
            ]
        })
    );
}

#[test]
fn parse_ast_supports_struct_patterns() {
    let tokens = scan_tokens(
        "defmodule User do\n  defstruct name: \"\", age: 0\n\n  def run(value) do\n    case value do\n      %User{name: name} -> name\n      _ -> \"none\"\n    end\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"case",
            "subject":{"kind":"variable","name":"value"},
            "branches":[
                {
                    "pattern":{
                        "kind":"struct",
                        "module":"User",
                        "entries":[
                            {"key":"name","value":{"kind":"bind","name":"name"}}
                        ]
                    },
                    "body":{"kind":"variable","name":"name"}
                },
                {
                    "pattern":{"kind":"wildcard"},
                    "body":{"kind":"string","value":"none"}
                }
            ]
        })
    );
}

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
fn parse_ast_assigns_stable_node_ids() {
    let tokens = scan_tokens(
        "defmodule Math do\n  def one() do\n    1\n  end\n\n  def two() do\n    one()\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let first = parse_ast(&tokens).expect("parser should produce ast");
    let second = parse_ast(&tokens).expect("parser should produce ast");

    let first_ids = collect_node_ids(&first);
    let second_ids = collect_node_ids(&second);

    assert_eq!(
        first_ids,
        [
            "module-0001",
            "function-0002",
            "expr-0003",
            "function-0004",
            "expr-0005",
        ]
    );
    assert_eq!(first_ids, second_ids);

    let unique_count = first_ids
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len();

    assert_eq!(unique_count, first_ids.len());
}

#[test]
fn parse_ast_supports_module_forms_and_attributes() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  alias Math, as: M\n  import Math\n  require Logger\n  use Feature\n  @moduledoc \"demo module\"\n  @doc \"run docs\"\n  @answer 5\n\n  def run() do\n    M.helper() + helper()\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[0].forms).expect("module forms should serialize"),
        serde_json::json!([
            {"kind":"alias","module":"Math","as":"M"},
            {"kind":"import","module":"Math"},
            {"kind":"require","module":"Logger"},
            {"kind":"use","module":"Feature"}
        ])
    );
    assert_eq!(
        serde_json::to_value(&ast.modules[0].attributes)
            .expect("module attributes should serialize"),
        serde_json::json!([
            {"name":"moduledoc","value":{"kind":"string","value":"demo module"}},
            {"name":"doc","value":{"kind":"string","value":"run docs"}},
            {"name":"answer","value":{"kind":"int","value":5}}
        ])
    );
    assert_eq!(
        serde_json::to_value(&ast.modules[0].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({
            "kind":"binary",
            "op":"plus",
            "left":{"kind":"call","callee":"Math.helper","args":[]},
            "right":{"kind":"call","callee":"Math.helper","args":[]}
        })
    );
}

#[test]
fn parse_ast_canonicalizes_use_calls_when_no_explicit_imports() {
    let tokens = scan_tokens(
        "defmodule Feature do\n  def helper() do\n    41\n  end\nend\n\ndefmodule Demo do\n  use Feature\n\n  def run() do\n    helper()\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[1].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({"kind":"call","callee":"Feature.helper","args":[]})
    );
}

#[test]
fn parse_ast_rejects_unsupported_alias_options() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  alias Math, via: M\n\n  def run() do\n    1\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject unsupported alias options");

    assert_eq!(
        error.to_string(),
        "unsupported alias option 'via'; supported syntax: alias Module, as: Name at offset 32"
    );
}

#[test]
fn parse_ast_supports_import_only_and_except_filters() {
    let tokens = scan_tokens(
        "defmodule Math do\n  def add(value, other) do\n    value + other\n  end\n\n  def unsafe(value) do\n    value - 1\n  end\nend\n\ndefmodule Demo do\n  import Math, only: [add: 2]\n\n  def run() do\n    add(20, 22)\n  end\nend\n\ndefmodule SafeDemo do\n  import Math, except: [unsafe: 1]\n\n  def run() do\n    add(2, 3)\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let ast = parse_ast(&tokens).expect("parser should produce ast");

    assert_eq!(
        serde_json::to_value(&ast.modules[1].forms).expect("module forms should serialize"),
        serde_json::json!([
            {
                "kind":"import",
                "module":"Math",
                "only":[{"name":"add","arity":2}]
            }
        ])
    );
    assert_eq!(
        serde_json::to_value(&ast.modules[1].functions[0].body)
            .expect("expression should serialize"),
        serde_json::json!({"kind":"call","callee":"Math.add","args":[{"kind":"int","value":20},{"kind":"int","value":22}]})
    );
    assert_eq!(
        serde_json::to_value(&ast.modules[2].forms).expect("module forms should serialize"),
        serde_json::json!([
            {
                "kind":"import",
                "module":"Math",
                "except":[{"name":"unsafe","arity":1}]
            }
        ])
    );
}

#[test]
fn parse_ast_rejects_malformed_import_filter_options() {
    let tokens = scan_tokens(
        "defmodule Demo do\n  import Math, only: [helper]\n\n  def run() do\n    helper(1)\n  end\nend\n",
    )
    .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens)
        .expect_err("parser should reject malformed import filter option payload");

    assert!(
        error
            .to_string()
            .starts_with("invalid import only option; expected only: [name: arity, ...] at offset"),
        "unexpected parser error: {error}"
    );
}

#[test]
fn parse_ast_reports_missing_module_end() {
    let tokens = scan_tokens("defmodule Broken do\n  def one() do\n    1\n  end\n")
        .expect("scanner should tokenize parser fixture");

    let error = parse_ast(&tokens).expect_err("parser should reject missing end");

    assert!(
        error
            .to_string()
            .starts_with("expected module declaration, found EOF"),
        "unexpected parser error: {error}"
    );
}

fn collect_node_ids(ast: &super::Ast) -> Vec<String> {
    let mut ids = Vec::new();

    for module in &ast.modules {
        ids.push(module.id.0.clone());

        for function in &module.functions {
            ids.push(function.id.0.clone());
            for param in &function.params {
                if let Some(default) = param.default() {
                    collect_expr_ids(default, &mut ids);
                }
            }
            if let Some(guard) = function.guard() {
                collect_expr_ids(guard, &mut ids);
            }
            collect_expr_ids(&function.body, &mut ids);
        }
    }

    ids
}

fn collect_expr_ids(expr: &Expr, ids: &mut Vec<String>) {
    match expr {
        Expr::Int { id, .. }
        | Expr::Float { id, .. }
        | Expr::Bool { id, .. }
        | Expr::Nil { id, .. }
        | Expr::String { id, .. } => ids.push(id.0.clone()),
        Expr::InterpolatedString { id, segments, .. } => {
            ids.push(id.0.clone());
            for segment in segments {
                if let crate::parser::InterpolationSegment::Expr { expr } = segment {
                    collect_expr_ids(expr, ids);
                }
            }
        }
        Expr::Tuple { id, items, .. }
        | Expr::List { id, items, .. }
        | Expr::Bitstring { id, items, .. } => {
            ids.push(id.0.clone());

            for item in items {
                collect_expr_ids(item, ids);
            }
        }
        Expr::Map { id, entries, .. } => {
            ids.push(id.0.clone());

            for entry in entries {
                collect_expr_ids(&entry.key, ids);
                collect_expr_ids(&entry.value, ids);
            }
        }
        Expr::Struct { id, entries, .. } => {
            ids.push(id.0.clone());

            for entry in entries {
                collect_expr_ids(&entry.value, ids);
            }
        }
        Expr::Keyword { id, entries, .. } => {
            ids.push(id.0.clone());

            for entry in entries {
                collect_expr_ids(&entry.value, ids);
            }
        }
        Expr::MapUpdate {
            id, base, updates, ..
        }
        | Expr::StructUpdate {
            id, base, updates, ..
        } => {
            ids.push(id.0.clone());
            collect_expr_ids(base, ids);
            for entry in updates {
                collect_expr_ids(&entry.value, ids);
            }
        }
        Expr::FieldAccess { id, base, .. } => {
            ids.push(id.0.clone());
            collect_expr_ids(base, ids);
        }
        Expr::IndexAccess {
            id, base, index, ..
        } => {
            ids.push(id.0.clone());
            collect_expr_ids(base, ids);
            collect_expr_ids(index, ids);
        }
        Expr::Call { id, args, .. } => {
            ids.push(id.0.clone());

            for arg in args {
                collect_expr_ids(arg, ids);
            }
        }
        Expr::Fn { id, body, .. } => {
            ids.push(id.0.clone());
            collect_expr_ids(body, ids);
        }
        Expr::Invoke {
            id, callee, args, ..
        } => {
            ids.push(id.0.clone());
            collect_expr_ids(callee, ids);
            for arg in args {
                collect_expr_ids(arg, ids);
            }
        }
        Expr::Question { id, value, .. } => {
            ids.push(id.0.clone());
            collect_expr_ids(value, ids);
        }
        Expr::Binary {
            id, left, right, ..
        } => {
            ids.push(id.0.clone());
            collect_expr_ids(left, ids);
            collect_expr_ids(right, ids);
        }
        Expr::Unary { id, value, .. } => {
            ids.push(id.0.clone());
            collect_expr_ids(value, ids);
        }
        Expr::Pipe {
            id, left, right, ..
        } => {
            ids.push(id.0.clone());
            collect_expr_ids(left, ids);
            collect_expr_ids(right, ids);
        }
        Expr::Case {
            id,
            subject,
            branches,
            ..
        } => {
            ids.push(id.0.clone());
            collect_expr_ids(subject, ids);

            for branch in branches {
                if let Some(guard) = branch.guard() {
                    collect_expr_ids(guard, ids);
                }
                collect_expr_ids(branch.body(), ids);
            }
        }
        Expr::For {
            id,
            generators,
            into,
            reduce,
            body,
            ..
        } => {
            ids.push(id.0.clone());
            for generator in generators {
                collect_expr_ids(generator.source(), ids);
                if let Some(guard) = generator.guard() {
                    collect_expr_ids(guard, ids);
                }
            }
            if let Some(into_expr) = into {
                collect_expr_ids(into_expr, ids);
            }
            if let Some(reduce_expr) = reduce {
                collect_expr_ids(reduce_expr, ids);
            }
            collect_expr_ids(body, ids);
        }
        Expr::Group { id, inner, .. } => {
            ids.push(id.0.clone());
            collect_expr_ids(inner, ids);
        }
        Expr::Try {
            id,
            body,
            rescue,
            catch,
            after,
            ..
        } => {
            ids.push(id.0.clone());
            collect_expr_ids(body, ids);
            for branch in rescue {
                if let Some(guard) = &branch.guard {
                    collect_expr_ids(guard, ids);
                }
                collect_expr_ids(&branch.body, ids);
            }
            for branch in catch {
                if let Some(guard) = &branch.guard {
                    collect_expr_ids(guard, ids);
                }
                collect_expr_ids(&branch.body, ids);
            }
            if let Some(after) = after {
                collect_expr_ids(after, ids);
            }
        }
        Expr::Raise { id, error, .. } => {
            ids.push(id.0.clone());
            collect_expr_ids(error, ids);
        }
        Expr::Variable { id, .. } | Expr::Atom { id, .. } => {
            ids.push(id.0.clone());
        }
        Expr::Block { id, exprs, .. } => {
            ids.push(id.0.clone());
            for sub_expr in exprs {
                collect_expr_ids(sub_expr, ids);
            }
        }
    }
}
