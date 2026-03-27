#![allow(dead_code)]

use super::algebra::{self, Doc};
use crate::lexer::scan_tokens;
use crate::parser::{
    parse_ast, Ast, BinaryOp, CaseBranch, Expr, Function, LabelExprEntry, LabelPatternEntry,
    MapExprEntry, MapPatternEntry, Module, ModuleForm, Parameter, ParameterAnnotation, Pattern,
    StructFieldEntry, UnaryOp,
};

pub(crate) fn format_parsed_source(source: &str, max_width: usize) -> Result<String, String> {
    let tokens = scan_tokens(source).map_err(|error| format!("lexer error: {error}"))?;
    let ast = parse_ast(&tokens).map_err(|error| format!("parser error: {error}"))?;
    format_ast(&ast, max_width)
}

pub(crate) fn format_ast(ast: &Ast, max_width: usize) -> Result<String, String> {
    let doc = ast_to_doc(ast)?;
    let output = algebra::format(&doc, max_width);
    let mut output = output
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

fn ast_to_doc(ast: &Ast) -> Result<Doc, String> {
    join_docs(
        ast.modules
            .iter()
            .map(module_to_doc)
            .collect::<Result<Vec<_>, _>>()?,
        blank_line(),
    )
}

fn module_to_doc(module: &Module) -> Result<Doc, String> {
    if !module.attributes.is_empty() {
        return Err(format!(
            "slice 5 formatter does not render module attributes yet: {}",
            module.name
        ));
    }

    let mut items = module
        .forms
        .iter()
        .map(module_form_to_doc)
        .collect::<Result<Vec<_>, _>>()?;
    items.extend(
        module
            .functions
            .iter()
            .map(function_to_doc)
            .collect::<Result<Vec<_>, _>>()?,
    );

    if items.is_empty() {
        return Ok(concat_all(vec![
            text(format!("defmodule {} do", module.name)),
            line(),
            text("end"),
        ]));
    }

    let body = join_docs(items, blank_line())?;

    Ok(concat_all(vec![
        text(format!("defmodule {} do", module.name)),
        nest(2, concat(line(), body)),
        line(),
        text("end"),
    ]))
}

fn module_form_to_doc(form: &ModuleForm) -> Result<Doc, String> {
    match form {
        ModuleForm::Defstruct { fields } => defstruct_to_doc(fields),
        unsupported => Err(format!(
            "slice 5 formatter does not render this module form yet: {unsupported:?}"
        )),
    }
}

fn defstruct_to_doc(fields: &[StructFieldEntry]) -> Result<Doc, String> {
    if fields.is_empty() {
        return Ok(text("defstruct []"));
    }

    let fields = join_docs(
        fields
            .iter()
            .map(struct_field_to_doc)
            .collect::<Result<Vec<_>, _>>()?,
        concat(text(","), line()),
    )?;

    Ok(group(concat_all(vec![
        text("defstruct"),
        nest(2, concat(line(), fields)),
    ])))
}

fn struct_field_to_doc(field: &StructFieldEntry) -> Result<Doc, String> {
    Ok(concat_all(vec![
        text(format!("{}: ", field.name)),
        expr_to_doc(&field.default, 0)?,
    ]))
}

fn function_to_doc(function: &Function) -> Result<Doc, String> {
    let keyword = if function.is_private() { "defp" } else { "def" };
    let params = join_docs(
        function
            .params
            .iter()
            .map(parameter_to_doc)
            .collect::<Result<Vec<_>, _>>()?,
        text(", "),
    )?;

    let mut head_parts = vec![text(format!("{keyword} {}(", function.name)), params, text(")")];
    if let Some(guard) = function.guard() {
        head_parts.push(text(" when "));
        head_parts.push(expr_to_doc(guard, 0)?);
    }
    head_parts.push(text(" do"));

    Ok(concat_all(vec![
        concat_all(head_parts),
        nest(2, concat(line(), expr_to_doc(&function.body, 0)?)),
        line(),
        text("end"),
    ]))
}

fn parameter_to_doc(parameter: &Parameter) -> Result<Doc, String> {
    match parameter.pattern() {
        Pattern::Bind { name } if name == parameter.name() => {}
        _ => {
            return Err(format!(
                "slice 3 formatter only supports identifier parameters, found unsupported pattern for {}",
                parameter.name()
            ));
        }
    }

    let base = match parameter.annotation() {
        ParameterAnnotation::Inferred => text(parameter.name()),
        ParameterAnnotation::Dynamic => text(format!("dynamic {}", parameter.name())),
    };

    if let Some(default) = parameter.default() {
        Ok(concat_all(vec![
            base,
            text(" \\\\ "),
            expr_to_doc(default, 0)?,
        ]))
    } else {
        Ok(base)
    }
}

fn expr_to_doc(expr: &Expr, parent_precedence: u8) -> Result<Doc, String> {
    let precedence = expr_precedence(expr);
    let doc = match expr {
        Expr::Int { value, .. } => text(value.to_string()),
        Expr::Float { value, .. } => text(value.clone()),
        Expr::Bool { value, .. } => text(if *value { "true" } else { "false" }),
        Expr::Nil { .. } => text("nil"),
        Expr::String { value, .. } => text(render_string_literal(value)),
        Expr::Variable { name, .. } => text(name),
        Expr::Atom { value, .. } => text(format!(":{value}")),
        Expr::Tuple { items, .. } => delimited_doc("{", items, "}")?,
        Expr::List { items, .. } => delimited_doc("[", items, "]")?,
        Expr::Keyword { entries, .. } => {
            bracketed_entries_doc("[", label_entries_to_docs(entries)?, "]")?
        }
        Expr::Map { entries, .. } => bracketed_entries_doc("%{", map_entries_to_docs(entries)?, "}")?,
        Expr::Struct {
            module, entries, ..
        } => bracketed_entries_doc(format!("%{}{{", module), label_entries_to_docs(entries)?, "}")?,
        Expr::MapUpdate { base, updates, .. } => update_doc("%{", base, updates, "}")?,
        Expr::StructUpdate {
            module,
            base,
            updates,
            ..
        } => update_doc(format!("%{}{{", module), base, updates, "}")?,
        Expr::Call { callee, args, .. } => call_doc(text(callee), args, false)?,
        Expr::Invoke { callee, args, .. } => call_doc(
            concat_all(vec![expr_to_doc(callee, PRECEDENCE_CALL)?, text(".(")]),
            args,
            true,
        )?,
        Expr::FieldAccess { base, label, .. } => concat_all(vec![
            expr_to_doc(base, PRECEDENCE_CALL)?,
            text("."),
            text(label),
        ]),
        Expr::IndexAccess { base, index, .. } => concat_all(vec![
            expr_to_doc(base, PRECEDENCE_CALL)?,
            text("["),
            expr_to_doc(index, 0)?,
            text("]"),
        ]),
        Expr::Question { value, .. } => {
            concat_all(vec![expr_to_doc(value, PRECEDENCE_CALL)?, text("?")])
        }
        Expr::Group { inner, .. } => concat_all(vec![text("("), expr_to_doc(inner, 0)?, text(")")]),
        Expr::Unary { op, value, .. } => unary_doc(*op, value)?,
        Expr::Binary {
            op, left, right, ..
        } => binary_doc(*op, left, right)?,
        Expr::Pipe { .. } => pipe_doc(expr)?,
        Expr::Case {
            subject, branches, ..
        } => case_doc(subject, branches)?,
        Expr::Block { exprs, .. } => join_docs(
            exprs
                .iter()
                .map(|expr| expr_to_doc(expr, 0))
                .collect::<Result<Vec<_>, _>>()?,
            line(),
        )?,
        unsupported => {
            return Err(format!(
                "slice 5 formatter does not render this expression yet: {unsupported:?}"
            ));
        }
    };

    if precedence < parent_precedence {
        Ok(parenthesize(doc))
    } else {
        Ok(doc)
    }
}

fn case_doc(subject: &Expr, branches: &[CaseBranch]) -> Result<Doc, String> {
    let branches = join_docs(
        branches
            .iter()
            .map(case_branch_to_doc)
            .collect::<Result<Vec<_>, _>>()?,
        line(),
    )?;

    Ok(concat_all(vec![
        text("case "),
        expr_to_doc(subject, 0)?,
        text(" do"),
        nest(2, concat(line(), branches)),
        line(),
        text("end"),
    ]))
}

fn case_branch_to_doc(branch: &CaseBranch) -> Result<Doc, String> {
    let mut head = vec![pattern_to_doc(branch.head())?];
    if let Some(guard) = branch.guard() {
        head.push(text(" when "));
        head.push(expr_to_doc(guard, 0)?);
    }
    head.push(text(" ->"));

    Ok(concat_all(vec![
        concat_all(head),
        nest(2, concat(line(), expr_to_doc(branch.body(), 0)?)),
    ]))
}

fn pattern_to_doc(pattern: &Pattern) -> Result<Doc, String> {
    match pattern {
        Pattern::Atom { value } => Ok(text(format!(":{value}"))),
        Pattern::Bind { name } => Ok(text(name)),
        Pattern::Pin { name } => Ok(text(format!("^{name}"))),
        Pattern::Wildcard => Ok(text("_")),
        Pattern::Integer { value } => Ok(text(value.to_string())),
        Pattern::Bool { value } => Ok(text(if *value { "true" } else { "false" })),
        Pattern::Nil => Ok(text("nil")),
        Pattern::String { value } => Ok(text(render_string_literal(value))),
        Pattern::Tuple { items } => pattern_delimited_doc("{", items, "}"),
        Pattern::List { items, tail } => list_pattern_doc(items, tail.as_deref()),
        Pattern::Map { entries } => {
            bracketed_entries_doc("%{", map_pattern_entries_to_docs(entries)?, "}")
        }
        Pattern::Struct { module, entries } => bracketed_entries_doc(
            format!("%{}{{", module),
            label_pattern_entries_to_docs(entries)?,
            "}",
        ),
        unsupported => Err(format!(
            "slice 5 formatter does not render this pattern yet: {unsupported:?}"
        )),
    }
}

fn pattern_delimited_doc(
    open: impl Into<String>,
    items: &[Pattern],
    close: impl Into<String>,
) -> Result<Doc, String> {
    bracketed_entries_doc(
        open,
        items
            .iter()
            .map(pattern_to_doc)
            .collect::<Result<Vec<_>, _>>()?,
        close,
    )
}

fn list_pattern_doc(items: &[Pattern], tail: Option<&Pattern>) -> Result<Doc, String> {
    let items = items
        .iter()
        .map(pattern_to_doc)
        .collect::<Result<Vec<_>, _>>()?;

    let Some(tail) = tail else {
        return bracketed_entries_doc("[", items, "]");
    };

    let mut body = Vec::new();
    if !items.is_empty() {
        body.push(join_docs(items, concat(text(","), line()))?);
        body.push(line());
    }
    body.push(concat(text("| "), pattern_to_doc(tail)?));

    Ok(group(concat_all(vec![
        text("["),
        nest(2, concat(soft_line(), concat_all(body))),
        soft_line(),
        text("]"),
    ])))
}

fn delimited_doc(open: impl Into<String>, items: &[Expr], close: impl Into<String>) -> Result<Doc, String> {
    bracketed_entries_doc(
        open,
        items
            .iter()
            .map(|item| expr_to_doc(item, 0))
            .collect::<Result<Vec<_>, _>>()?,
        close,
    )
}

fn bracketed_entries_doc(
    open: impl Into<String>,
    entries: Vec<Doc>,
    close: impl Into<String>,
) -> Result<Doc, String> {
    let open = open.into();
    let close = close.into();

    if entries.is_empty() {
        return Ok(concat_all(vec![text(open), text(close)]));
    }

    let entries = join_docs(entries, concat(text(","), line()))?;

    Ok(group(concat_all(vec![
        text(open),
        nest(2, concat(soft_line(), entries)),
        soft_line(),
        text(close),
    ])))
}

fn update_doc(
    open: impl Into<String>,
    base: &Expr,
    updates: &[LabelExprEntry],
    close: impl Into<String>,
) -> Result<Doc, String> {
    let open = open.into();
    let close = close.into();

    let base = expr_to_doc(base, 0)?;
    if updates.is_empty() {
        return Ok(group(concat_all(vec![
            text(open),
            nest(2, concat(soft_line(), base)),
            soft_line(),
            text(close),
        ])));
    }

    let updates = join_docs(
        updates
            .iter()
            .map(label_entry_to_doc)
            .collect::<Result<Vec<_>, _>>()?,
        concat(text(","), line()),
    )?;
    let updates = concat(text("| "), updates);

    Ok(group(concat_all(vec![
        text(open),
        nest(
            2,
            concat_all(vec![soft_line(), base, line(), nest(2, updates)]),
        ),
        soft_line(),
        text(close),
    ])))
}

fn map_entries_to_docs(entries: &[MapExprEntry]) -> Result<Vec<Doc>, String> {
    entries.iter().map(map_entry_to_doc).collect()
}

fn map_entry_to_doc(entry: &MapExprEntry) -> Result<Doc, String> {
    match entry.key() {
        Expr::Atom { value, .. } => Ok(concat_all(vec![
            text(format!("{value}: ")),
            expr_to_doc(entry.value(), 0)?,
        ])),
        key => Ok(concat_all(vec![
            expr_to_doc(key, 0)?,
            text(" => "),
            expr_to_doc(entry.value(), 0)?,
        ])),
    }
}

fn map_pattern_entries_to_docs(entries: &[MapPatternEntry]) -> Result<Vec<Doc>, String> {
    entries.iter().map(map_pattern_entry_to_doc).collect()
}

fn map_pattern_entry_to_doc(entry: &MapPatternEntry) -> Result<Doc, String> {
    match entry.key() {
        Pattern::Atom { value } => Ok(concat_all(vec![
            text(format!("{value}: ")),
            pattern_to_doc(entry.value())?,
        ])),
        key => Ok(concat_all(vec![
            pattern_to_doc(key)?,
            text(" => "),
            pattern_to_doc(entry.value())?,
        ])),
    }
}

fn label_entries_to_docs(entries: &[LabelExprEntry]) -> Result<Vec<Doc>, String> {
    entries.iter().map(label_entry_to_doc).collect()
}

fn label_entry_to_doc(entry: &LabelExprEntry) -> Result<Doc, String> {
    Ok(concat_all(vec![
        text(format!("{}: ", entry.key)),
        expr_to_doc(&entry.value, 0)?,
    ]))
}

fn label_pattern_entries_to_docs(entries: &[LabelPatternEntry]) -> Result<Vec<Doc>, String> {
    entries.iter().map(label_pattern_entry_to_doc).collect()
}

fn label_pattern_entry_to_doc(entry: &LabelPatternEntry) -> Result<Doc, String> {
    Ok(concat_all(vec![
        text(format!("{}: ", entry.key())),
        pattern_to_doc(entry.value())?,
    ]))
}

fn call_doc(head: Doc, args: &[Expr], head_includes_open_paren: bool) -> Result<Doc, String> {
    if args.is_empty() {
        return Ok(if head_includes_open_paren {
            concat_all(vec![head, text(")")])
        } else {
            concat_all(vec![head, text("()")])
        });
    }

    let open = if head_includes_open_paren {
        head
    } else {
        concat_all(vec![head, text("(")])
    };
    let args_doc = join_docs(
        args.iter()
            .map(|arg| expr_to_doc(arg, 0))
            .collect::<Result<Vec<_>, _>>()?,
        concat(text(","), line()),
    )?;

    Ok(group(concat_all(vec![
        open,
        nest(2, concat(soft_line(), args_doc)),
        soft_line(),
        text(")"),
    ])))
}

fn unary_doc(op: UnaryOp, value: &Expr) -> Result<Doc, String> {
    let (operator, spaced) = match op {
        UnaryOp::Not => ("not", true),
        UnaryOp::Bang => ("!", false),
        UnaryOp::Plus => ("+", false),
        UnaryOp::Minus => ("-", false),
        UnaryOp::BitwiseNot => ("~~~", false),
    };

    let value = expr_to_doc(value, PRECEDENCE_UNARY)?;
    Ok(if spaced {
        concat_all(vec![text(operator), text(" "), value])
    } else {
        concat_all(vec![text(operator), value])
    })
}

fn binary_doc(op: BinaryOp, left: &Expr, right: &Expr) -> Result<Doc, String> {
    let precedence = binary_precedence(op);
    Ok(concat_all(vec![
        expr_to_doc(left, precedence)?,
        text(format!(" {} ", binary_op_text(op))),
        expr_to_doc(right, precedence + 1)?,
    ]))
}

fn pipe_doc(expr: &Expr) -> Result<Doc, String> {
    let mut segments = Vec::new();
    collect_pipe_segments(expr, &mut segments);

    let mut docs = Vec::new();
    for (index, segment) in segments.into_iter().enumerate() {
        if index > 0 {
            docs.push(line());
            docs.push(text("|> "));
        }
        docs.push(expr_to_doc(segment, PRECEDENCE_PIPE + 1)?);
    }

    Ok(group(concat_all(docs)))
}

fn collect_pipe_segments<'a>(expr: &'a Expr, output: &mut Vec<&'a Expr>) {
    if let Expr::Pipe { left, right, .. } = expr {
        collect_pipe_segments(left, output);
        output.push(right.as_ref());
    } else {
        output.push(expr);
    }
}

const PRECEDENCE_PIPE: u8 = 1;
const PRECEDENCE_MATCH: u8 = 2;
const PRECEDENCE_LOGICAL_OR: u8 = 3;
const PRECEDENCE_LOGICAL_AND: u8 = 4;
const PRECEDENCE_COMPARISON: u8 = 5;
const PRECEDENCE_CONCAT: u8 = 6;
const PRECEDENCE_ADDITIVE: u8 = 7;
const PRECEDENCE_MULTIPLICATIVE: u8 = 8;
const PRECEDENCE_UNARY: u8 = 9;
const PRECEDENCE_CALL: u8 = 10;
const PRECEDENCE_ATOM: u8 = 11;

fn expr_precedence(expr: &Expr) -> u8 {
    match expr {
        Expr::Pipe { .. } => PRECEDENCE_PIPE,
        Expr::Binary { op, .. } => binary_precedence(*op),
        Expr::Unary { .. } => PRECEDENCE_UNARY,
        Expr::Call { .. }
        | Expr::Invoke { .. }
        | Expr::FieldAccess { .. }
        | Expr::IndexAccess { .. }
        | Expr::Question { .. } => PRECEDENCE_CALL,
        _ => PRECEDENCE_ATOM,
    }
}

fn binary_precedence(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Match => PRECEDENCE_MATCH,
        BinaryOp::Or | BinaryOp::OrOr => PRECEDENCE_LOGICAL_OR,
        BinaryOp::And | BinaryOp::AndAnd => PRECEDENCE_LOGICAL_AND,
        BinaryOp::Eq
        | BinaryOp::NotEq
        | BinaryOp::Lt
        | BinaryOp::Lte
        | BinaryOp::Gt
        | BinaryOp::Gte
        | BinaryOp::In
        | BinaryOp::NotIn
        | BinaryOp::StrictEq
        | BinaryOp::StrictBangEq => PRECEDENCE_COMPARISON,
        BinaryOp::Concat | BinaryOp::PlusPlus | BinaryOp::MinusMinus => PRECEDENCE_CONCAT,
        BinaryOp::Plus | BinaryOp::Minus | BinaryOp::Range | BinaryOp::SteppedRange => {
            PRECEDENCE_ADDITIVE
        }
        BinaryOp::Mul
        | BinaryOp::Div
        | BinaryOp::IntDiv
        | BinaryOp::Rem
        | BinaryOp::BitwiseAnd
        | BinaryOp::BitwiseOr
        | BinaryOp::BitwiseXor
        | BinaryOp::BitwiseShiftLeft
        | BinaryOp::BitwiseShiftRight => PRECEDENCE_MULTIPLICATIVE,
    }
}

fn binary_op_text(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Match => "=",
        BinaryOp::Plus => "+",
        BinaryOp::Minus => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Eq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::Lt => "<",
        BinaryOp::Lte => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::Gte => ">=",
        BinaryOp::And => "and",
        BinaryOp::Or => "or",
        BinaryOp::AndAnd => "&&",
        BinaryOp::OrOr => "||",
        BinaryOp::Concat => "<>",
        BinaryOp::PlusPlus => "++",
        BinaryOp::MinusMinus => "--",
        BinaryOp::In => "in",
        BinaryOp::NotIn => "not in",
        BinaryOp::Range => "..",
        BinaryOp::StrictEq => "===",
        BinaryOp::StrictBangEq => "!==",
        BinaryOp::BitwiseAnd => "&&&",
        BinaryOp::BitwiseOr => "|||",
        BinaryOp::BitwiseXor => "^^^",
        BinaryOp::BitwiseShiftLeft => "<<<",
        BinaryOp::BitwiseShiftRight => ">>>",
        BinaryOp::SteppedRange => "..//",
        BinaryOp::IntDiv => "//",
        BinaryOp::Rem => "rem",
    }
}

fn render_string_literal(value: &str) -> String {
    serde_json::to_string(value).expect("string literal should serialize")
}

fn parenthesize(doc: Doc) -> Doc {
    concat_all(vec![text("("), doc, text(")")])
}

fn blank_line() -> Doc {
    concat(line(), line())
}

fn text(value: impl Into<String>) -> Doc {
    Doc::Text(value.into())
}

fn line() -> Doc {
    Doc::Line
}

fn soft_line() -> Doc {
    Doc::SoftLine
}

fn concat(left: Doc, right: Doc) -> Doc {
    Doc::Concat(Box::new(left), Box::new(right))
}

fn nest(indent: i32, doc: Doc) -> Doc {
    Doc::Nest(indent, Box::new(doc))
}

fn group(doc: Doc) -> Doc {
    Doc::Group(Box::new(doc))
}

fn concat_all(docs: Vec<Doc>) -> Doc {
    let mut docs = docs.into_iter();
    let Some(mut doc) = docs.next() else {
        return Doc::Nil;
    };

    for next in docs {
        doc = concat(doc, next);
    }
    doc
}

fn join_docs(docs: Vec<Doc>, separator: Doc) -> Result<Doc, String> {
    let mut docs = docs.into_iter();
    let Some(mut doc) = docs.next() else {
        return Ok(Doc::Nil);
    };

    for next in docs {
        doc = concat_all(vec![doc, separator.clone(), next]);
    }

    Ok(doc)
}

#[cfg(test)]
mod tests {
    use super::format_parsed_source;

    #[test]
    fn format_ast_module_separates_sibling_functions() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def one() do\n",
            "    alpha()\n",
            "  end\n",
            "\n",
            "  def two() do\n",
            "    beta()\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 80).expect("ast formatter should render");

        assert_eq!(
            rendered,
            concat!(
                "defmodule Demo do\n",
                "  def one() do\n",
                "    alpha()\n",
                "  end\n",
                "\n",
                "  def two() do\n",
                "    beta()\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_renders_defstruct_before_functions() {
        let source = concat!(
            "defmodule User do\n",
            "  defstruct name: \"\", age: 0\n",
            "\n",
            "  def run() do\n",
            "    %User{name: \"A\"}\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 80).expect("ast formatter should render");

        assert_eq!(rendered, source);
    }

    #[test]
    fn format_ast_renders_private_function_defaults_and_guard() {
        let source = concat!(
            "defmodule Demo do\n",
            "  defp add(value, inc \\\\ 2) when inc > 0 do\n",
            "    value + inc\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 80).expect("ast formatter should render");

        assert_eq!(rendered, source);
    }

    #[test]
    fn format_ast_wraps_call_arguments_by_width() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def run() do\n",
            "    combine(alpha, beta, gamma, delta)\n",
            "  end\n",
            "end\n",
        );

        let wide = format_parsed_source(source, 80).expect("wide render should succeed");
        let narrow = format_parsed_source(source, 18).expect("narrow render should succeed");

        assert_eq!(wide, source);
        assert_eq!(
            narrow,
            concat!(
                "defmodule Demo do\n",
                "  def run() do\n",
                "    combine(\n",
                "      alpha,\n",
                "      beta,\n",
                "      gamma,\n",
                "      delta\n",
                "    )\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_wraps_tuple_and_list_literals_by_width() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def run() do\n",
            "    {:ok, [alpha, beta, gamma]}\n",
            "  end\n",
            "end\n",
        );

        let wide = format_parsed_source(source, 80).expect("wide render should succeed");
        let narrow = format_parsed_source(source, 16).expect("narrow render should succeed");

        assert_eq!(wide, source);
        assert_eq!(
            narrow,
            concat!(
                "defmodule Demo do\n",
                "  def run() do\n",
                "    {\n",
                "      :ok,\n",
                "      [\n",
                "        alpha,\n",
                "        beta,\n",
                "        gamma\n",
                "      ]\n",
                "    }\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_wraps_map_and_keyword_literals_by_width() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def run() do\n",
            "    {%{\"status\" => 200, ok: true, 1 => false}, [name: \"Ada\", age: 43]}\n",
            "  end\n",
            "end\n",
        );

        let wide = format_parsed_source(source, 80).expect("wide render should succeed");
        let narrow = format_parsed_source(source, 24).expect("narrow render should succeed");

        assert_eq!(wide, source);
        assert_eq!(
            narrow,
            concat!(
                "defmodule Demo do\n",
                "  def run() do\n",
                "    {\n",
                "      %{\n",
                "        \"status\" => 200,\n",
                "        ok: true,\n",
                "        1 => false\n",
                "      },\n",
                "      [\n",
                "        name: \"Ada\",\n",
                "        age: 43\n",
                "      ]\n",
                "    }\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_wraps_struct_and_update_literals_by_width() {
        let source = concat!(
            "defmodule User do\n",
            "  defstruct name: \"\", age: 0\n",
            "\n",
            "  def run(user, data) do\n",
            "    {%User{name: \"A\", age: 43}, %{data | age: 44, name: \"Ada\"}, %User{user | age: 45, name: \"Ada\"}}\n",
            "  end\n",
            "end\n",
        );

        let wide = format_parsed_source(source, 120).expect("wide render should succeed");
        let narrow = format_parsed_source(source, 26).expect("narrow render should succeed");

        assert_eq!(wide, source);
        assert_eq!(
            narrow,
            concat!(
                "defmodule User do\n",
                "  defstruct\n",
                "    name: \"\",\n",
                "    age: 0\n",
                "\n",
                "  def run(user, data) do\n",
                "    {\n",
                "      %User{\n",
                "        name: \"A\",\n",
                "        age: 43\n",
                "      },\n",
                "      %{\n",
                "        data\n",
                "        | age: 44,\n",
                "          name: \"Ada\"\n",
                "      },\n",
                "      %User{\n",
                "        user\n",
                "        | age: 45,\n",
                "          name: \"Ada\"\n",
                "      }\n",
                "    }\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_breaks_pipe_chains_before_each_pipe() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def run() do\n",
            "    source() |> normalize() |> persist()\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 18).expect("pipe render should succeed");

        assert_eq!(
            rendered,
            concat!(
                "defmodule Demo do\n",
                "  def run() do\n",
                "    source()\n",
                "    |> normalize()\n",
                "    |> persist()\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_preserves_block_body_line_structure() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def run() do\n",
            "    first(alpha)\n",
            "    second(beta)\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 80).expect("block render should succeed");

        assert_eq!(rendered, source);
    }

    #[test]
    fn format_ast_renders_case_patterns_guards_and_nested_case_bodies() {
        let source = concat!(
            "defmodule User do\n",
            "  defstruct name: \"\", age: 0\n",
            "\n",
            "  def run(expected, value) do\n",
            "    case value do\n",
            "      {:ok, result} -> result\n",
            "      [^expected | tail] when expected > 0 -> tail\n",
            "      %{\"status\" => code, ok: flag} -> case flag do\n",
            "        true -> {code, flag}\n",
            "        _ -> nil\n",
            "      end\n",
            "      %User{name: name, age: age} -> {name, age}\n",
            "      _ -> nil\n",
            "    end\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 120).expect("case render should succeed");

        assert_eq!(
            rendered,
            concat!(
                "defmodule User do\n",
                "  defstruct name: \"\", age: 0\n",
                "\n",
                "  def run(expected, value) do\n",
                "    case value do\n",
                "      {:ok, result} ->\n",
                "        result\n",
                "      [^expected | tail] when expected > 0 ->\n",
                "        tail\n",
                "      %{\"status\" => code, ok: flag} ->\n",
                "        case flag do\n",
                "          true ->\n",
                "            {code, flag}\n",
                "          _ ->\n",
                "            nil\n",
                "        end\n",
                "      %User{name: name, age: age} ->\n",
                "        {name, age}\n",
                "      _ ->\n",
                "        nil\n",
                "    end\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_lowers_if_and_unless_to_case() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def pick(flag) do\n",
            "    if flag do\n",
            "      1\n",
            "    else\n",
            "      0\n",
            "    end\n",
            "  end\n",
            "\n",
            "  def reject(flag) do\n",
            "    unless flag do\n",
            "      2\n",
            "    else\n",
            "      3\n",
            "    end\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 80).expect("control render should succeed");

        assert_eq!(
            rendered,
            concat!(
                "defmodule Demo do\n",
                "  def pick(flag) do\n",
                "    case nil do\n",
                "      _ when !!flag ->\n",
                "        1\n",
                "      _ ->\n",
                "        0\n",
                "    end\n",
                "  end\n",
                "\n",
                "  def reject(flag) do\n",
                "    case nil do\n",
                "      _ when !!flag ->\n",
                "        3\n",
                "      _ ->\n",
                "        2\n",
                "    end\n",
                "  end\n",
                "end\n",
            )
        );
    }

    #[test]
    fn format_ast_lowers_cond_and_with_to_nested_cases() {
        let source = concat!(
            "defmodule Demo do\n",
            "  def route(value) do\n",
            "    cond do\n",
            "      value > 2 -> 4\n",
            "      true -> 5\n",
            "    end\n",
            "  end\n",
            "\n",
            "  def chain() do\n",
            "    with [left, right] <- list(1, 2),\n",
            "         total <- left + right do\n",
            "      total\n",
            "    else\n",
            "      _ -> 0\n",
            "    end\n",
            "  end\n",
            "end\n",
        );

        let rendered = format_parsed_source(source, 80).expect("control render should succeed");

        assert_eq!(
            rendered,
            concat!(
                "defmodule Demo do\n",
                "  def route(value) do\n",
                "    case nil do\n",
                "      _ when !!(value > 2) ->\n",
                "        4\n",
                "      _ when !!true ->\n",
                "        5\n",
                "    end\n",
                "  end\n",
                "\n",
                "  def chain() do\n",
                "    case list(1, 2) do\n",
                "      [left, right] ->\n",
                "        case left + right do\n",
                "          total ->\n",
                "            total\n",
                "          __tonic_with_failure ->\n",
                "            case __tonic_with_failure do\n",
                "              _ ->\n",
                "                0\n",
                "            end\n",
                "        end\n",
                "      __tonic_with_failure ->\n",
                "        case __tonic_with_failure do\n",
                "          _ ->\n",
                "            0\n",
                "        end\n",
                "    end\n",
                "  end\n",
                "end\n",
            )
        );
    }
}
