use tower_lsp::lsp_types::{GotoDefinitionResponse, Location, Position, Range, Url};

use crate::lexer::{scan_tokens, TokenKind};
use crate::lsp::document::offset_to_position;

/// Attempt to find the definition of the symbol under `position` in `source`.
///
/// Strategy:
/// 1. Tokenize the source.
/// 2. Find the token at (or nearest to) the cursor byte offset.
/// 3. If it is an identifier, search backwards/forwards for a `def`/`defp`
///    declaration with that name in the same file.
pub fn find_definition(
    uri: &Url,
    source: &str,
    position: Position,
) -> Option<GotoDefinitionResponse> {
    let tokens = scan_tokens(source).ok()?;

    // Convert LSP position to byte offset.
    let cursor_offset = position_to_offset(source, position);

    // Find the identifier token at the cursor.
    let cursor_ident = tokens.iter().find(|t| {
        t.kind() == TokenKind::Ident
            && t.span().start() <= cursor_offset
            && cursor_offset <= t.span().end()
    })?;
    let target_name = cursor_ident.lexeme();

    // Scan for a def/defp declaration with that name.
    for window in tokens.windows(3) {
        let [kw, name_tok, _rest] = window else {
            continue;
        };
        let is_def = matches!(kw.kind(), TokenKind::Def | TokenKind::Defp);
        if !is_def {
            continue;
        }
        if name_tok.kind() != TokenKind::Ident {
            continue;
        }
        if name_tok.lexeme() != target_name {
            continue;
        }

        let def_offset = kw.span().start();
        let start = offset_to_position(source, def_offset);
        let end = offset_to_position(source, name_tok.span().end());
        let location = Location {
            uri: uri.clone(),
            range: Range { start, end },
        };
        return Some(GotoDefinitionResponse::Scalar(location));
    }

    None
}

/// Convert an LSP (line, character) position to a byte offset.
pub fn position_to_offset(source: &str, position: Position) -> usize {
    let mut current_line = 0u32;
    let mut line_start_byte = 0usize;

    for (i, ch) in source.char_indices() {
        if current_line == position.line {
            // Walk character-by-character within this line to find the column.
            for (col, (j, c)) in source[line_start_byte..].char_indices().enumerate() {
                if col as u32 == position.character {
                    return line_start_byte + j;
                }
                if c == '\n' {
                    break;
                }
            }
            // Column past end of line — return end of line.
            return line_start_byte
                + source[line_start_byte..]
                    .find('\n')
                    .unwrap_or(source.len() - line_start_byte);
        }
        if ch == '\n' {
            current_line += 1;
            line_start_byte = i + 1;
        }
    }

    source.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_uri() -> Url {
        Url::parse("file:///tmp/test.tn").unwrap()
    }

    #[test]
    fn find_definition_locates_function_in_same_file() {
        let source = "defmodule Demo do\n  def run() do\n    42\n  end\nend\n";
        // Cursor on "run" identifier in body — not a call here but let's ask for def of "run"
        // Line 1, character 6 is the "run" in `def run()`
        let position = Position {
            line: 1,
            character: 6,
        };
        let result = find_definition(&dummy_uri(), source, position);
        assert!(result.is_some(), "expected a definition location");
    }

    #[test]
    fn find_definition_returns_none_for_unknown_symbol() {
        let source = "defmodule Demo do\n  def run() do\n    42\n  end\nend\n";
        // Position on integer literal — not an identifier
        let position = Position {
            line: 2,
            character: 4,
        };
        let result = find_definition(&dummy_uri(), source, position);
        // Integer literal is not an ident so None is expected
        assert!(result.is_none());
    }
}
