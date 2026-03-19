use super::types::{LexerError, Span, Token, TokenKind};
use super::LexerState;

/// Scan a `"..."` or `"""..."""` string literal (or interpolation start) in Normal state.
///
/// Called when the current character is `"`. Handles heredocs, simple strings,
/// and strings with interpolation.
pub(super) fn scan_string_literal(
    chars: &[char],
    idx: &mut usize,
    tokens: &mut Vec<Token>,
    state_stack: &mut Vec<LexerState>,
    current_brace_depth: usize,
) -> Result<(), LexerError> {
    let start = *idx;
    let is_heredoc = chars.get(*idx + 1) == Some(&'"') && chars.get(*idx + 2) == Some(&'"');

    let mut has_interpolation = false;
    let mut temp_idx = if is_heredoc { *idx + 3 } else { *idx + 1 };

    if is_heredoc {
        while temp_idx < chars.len() {
            if chars.get(temp_idx) == Some(&'"')
                && chars.get(temp_idx + 1) == Some(&'"')
                && chars.get(temp_idx + 2) == Some(&'"')
            {
                break;
            }
            if chars.get(temp_idx) == Some(&'#') && chars.get(temp_idx + 1) == Some(&'{') {
                has_interpolation = true;
                break;
            }
            temp_idx += 1;
        }
    } else {
        while temp_idx < chars.len() {
            let peek = chars[temp_idx];
            if peek == '"' {
                break;
            }
            if peek == '#' && chars.get(temp_idx + 1) == Some(&'{') {
                has_interpolation = true;
                break;
            }
            temp_idx += 1;
        }
    }

    if has_interpolation {
        let end_idx = if is_heredoc { *idx + 3 } else { *idx + 1 };
        tokens.push(Token::simple(
            TokenKind::StringStart,
            Span::new(start, end_idx),
        ));
        state_stack.push(LexerState::String {
            is_heredoc,
            brace_depth: current_brace_depth,
        });
        *idx = end_idx;
    } else {
        let mut literal = String::new();
        let mut terminated = false;

        if is_heredoc {
            *idx += 3;

            while *idx < chars.len() {
                if chars.get(*idx) == Some(&'"')
                    && chars.get(*idx + 1) == Some(&'"')
                    && chars.get(*idx + 2) == Some(&'"')
                {
                    terminated = true;
                    *idx += 3;
                    break;
                }

                literal.push(chars[*idx]);
                *idx += 1;
            }
        } else {
            *idx += 1;

            while *idx < chars.len() {
                let peek = chars[*idx];

                if peek == '"' {
                    terminated = true;
                    *idx += 1;
                    break;
                }

                literal.push(peek);
                *idx += 1;
            }
        }

        if !terminated {
            return Err(LexerError::unterminated_string(Span::new(
                start,
                chars.len(),
            )));
        }

        tokens.push(Token::with_lexeme(
            TokenKind::String,
            literal,
            Span::new(start, *idx),
        ));
    }

    Ok(())
}

/// Scan string content when in `LexerState::String`.
///
/// Processes text until the closing quote, an interpolation `#{`, or end-of-input.
pub(super) fn scan_string_content(
    chars: &[char],
    idx: &mut usize,
    tokens: &mut Vec<Token>,
    state_stack: &mut Vec<LexerState>,
    current_brace_depth: &mut usize,
    is_heredoc: bool,
) -> Result<(), LexerError> {
    let start = *idx;
    let mut literal = String::new();
    let mut terminated = false;
    let mut is_interpolation = false;

    if is_heredoc {
        while *idx < chars.len() {
            if chars.get(*idx) == Some(&'"')
                && chars.get(*idx + 1) == Some(&'"')
                && chars.get(*idx + 2) == Some(&'"')
            {
                terminated = true;
                *idx += 3;
                break;
            }
            if chars.get(*idx) == Some(&'#') && chars.get(*idx + 1) == Some(&'{') {
                is_interpolation = true;
                *idx += 2;
                break;
            }

            literal.push(chars[*idx]);
            *idx += 1;
        }
    } else {
        while *idx < chars.len() {
            let peek = chars[*idx];

            if peek == '"' {
                terminated = true;
                *idx += 1;
                break;
            }
            if peek == '#' && chars.get(*idx + 1) == Some(&'{') {
                is_interpolation = true;
                *idx += 2;
                break;
            }

            literal.push(peek);
            *idx += 1;
        }
    }

    if !literal.is_empty() {
        let end_idx = if is_interpolation {
            *idx - 2
        } else if terminated && is_heredoc {
            *idx - 3
        } else if terminated {
            *idx - 1
        } else {
            *idx
        };
        tokens.push(Token::with_lexeme(
            TokenKind::StringPart,
            literal,
            Span::new(start, end_idx),
        ));
    }

    if terminated {
        tokens.push(Token::simple(
            TokenKind::StringEnd,
            Span::new(*idx - if is_heredoc { 3 } else { 1 }, *idx),
        ));
        state_stack.pop();
    } else if is_interpolation {
        tokens.push(Token::simple(
            TokenKind::InterpolationStart,
            Span::new(*idx - 2, *idx),
        ));
        state_stack.push(LexerState::Normal);
        *current_brace_depth += 1;
    } else {
        return Err(LexerError::unterminated_string(Span::new(
            start,
            chars.len(),
        )));
    }

    Ok(())
}

/// Scan a `~s(...)`, `~r/.../`, or `~w(...)` sigil in Normal state.
///
/// Also handles the `~~~` bitwise-not operator (returns early after emitting the token).
pub(super) fn scan_sigil(
    chars: &[char],
    idx: &mut usize,
    tokens: &mut Vec<Token>,
) -> Result<(), LexerError> {
    let start = *idx;

    // ~~~ is bitwise not (unary operator)
    if chars.get(*idx + 1) == Some(&'~') && chars.get(*idx + 2) == Some(&'~') {
        *idx += 3;
        tokens.push(Token::simple(
            TokenKind::TildeTildeTilde,
            Span::new(start, *idx),
        ));
        return Ok(());
    }

    let Some(sigil_kind) = chars.get(*idx + 1).copied() else {
        return Err(LexerError::invalid_token('~', Span::new(start, start + 1)));
    };

    if !matches!(sigil_kind, 's' | 'r' | 'w') {
        return Err(LexerError::invalid_token('~', Span::new(start, start + 1)));
    }

    let Some(open_delim) = chars.get(*idx + 2).copied() else {
        return Err(LexerError::invalid_token('~', Span::new(start, start + 1)));
    };

    let close_delim = match open_delim {
        '(' => ')',
        '[' => ']',
        '{' => '}',
        '<' => '>',
        other => other,
    };

    *idx += 3;
    let content_start = *idx;
    while *idx < chars.len() && chars[*idx] != close_delim {
        *idx += 1;
    }

    if *idx >= chars.len() {
        return Err(LexerError::unterminated_string(Span::new(
            start,
            chars.len(),
        )));
    }

    let lexeme: String = chars[content_start..*idx].iter().collect();
    *idx += 1;

    if sigil_kind == 'w' {
        // Check for optional modifier after closing delimiter (e.g. `a` for atoms)
        let use_atoms = chars.get(*idx) == Some(&'a');
        if use_atoms {
            *idx += 1;
        }

        // Split content on whitespace and emit a list literal token sequence
        let words: Vec<&str> = lexeme.split_whitespace().collect();
        tokens.push(Token::simple(TokenKind::LBracket, Span::new(start, *idx)));
        for (i, word) in words.iter().enumerate() {
            if i > 0 {
                tokens.push(Token::simple(TokenKind::Comma, Span::new(start, *idx)));
            }
            if use_atoms {
                tokens.push(Token::with_lexeme(
                    TokenKind::Atom,
                    word.to_string(),
                    Span::new(start, *idx),
                ));
            } else {
                tokens.push(Token::with_lexeme(
                    TokenKind::String,
                    word.to_string(),
                    Span::new(start, *idx),
                ));
            }
        }
        tokens.push(Token::simple(TokenKind::RBracket, Span::new(start, *idx)));
    } else {
        tokens.push(Token::with_lexeme(
            TokenKind::String,
            lexeme,
            Span::new(start, *idx),
        ));
    }

    Ok(())
}
