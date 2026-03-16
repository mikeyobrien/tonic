use super::{parse_ast, Expr};
use crate::lexer::scan_tokens;

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
