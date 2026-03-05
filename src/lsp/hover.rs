use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position};

use crate::lexer::{scan_tokens, TokenKind};
use crate::lsp::definition::position_to_offset;
use crate::parser::parse_ast;
use crate::typing::infer_types;

/// Return hover information for the symbol at `position` in `source`.
///
/// We find the identifier token at the cursor, then look up the type for a
/// matching `Module.function` signature in the type summary.
pub fn hover_info(source: &str, position: Position) -> Option<Hover> {
    let tokens = scan_tokens(source).ok()?;
    let ast = parse_ast(&tokens).ok()?;

    let cursor_offset = position_to_offset(source, position);

    // Find the identifier under the cursor.
    let token = tokens.iter().find(|t| {
        t.kind() == TokenKind::Ident
            && t.span().start() <= cursor_offset
            && cursor_offset <= t.span().end()
    })?;
    let symbol_name = token.lexeme().to_string();

    // Run type inference.
    let summary = infer_types(&ast).ok()?;

    // Try to match against `Module.symbol_name` for every module in the AST.
    for module in &ast.modules {
        let qualified = format!("{}.{}", module.name, symbol_name);
        if let Some(sig) = summary.lookup(&qualified) {
            let content = format!("```\ndef {} :: {}\n```", symbol_name, sig);
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: content,
                }),
                range: None,
            });
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hover_returns_type_for_function_name() {
        let source = "defmodule Demo do\n  def run() do\n    42\n  end\nend\n";
        // Position on "run" in `def run()`
        let position = Position {
            line: 1,
            character: 6,
        };
        let result = hover_info(source, position);
        assert!(result.is_some(), "expected hover content for 'run'");
    }

    #[test]
    fn hover_returns_none_for_non_function_token() {
        let source = "defmodule Demo do\n  def run() do\n    42\n  end\nend\n";
        // Position on integer "42"
        let position = Position {
            line: 2,
            character: 4,
        };
        let result = hover_info(source, position);
        // 42 is not an identifier — None expected
        assert!(result.is_none());
    }
}
