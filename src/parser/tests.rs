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
