use super::types::{LexerError, Span, Token, TokenKind};
use super::LexerState;

/// Process a backslash escape sequence, returning the replacement character.
/// Advances `idx` past the escape (caller already consumed the `\`).
fn process_escape(chars: &[char], idx: &mut usize) -> char {
    if *idx >= chars.len() {
        return '\\';
    }
    let next = chars[*idx];
    *idx += 1;
    match next {
        'n' => '\n',
        't' => '\t',
        'r' => '\r',
        '\\' => '\\',
        '"' => '"',
        _other => {
            // Unknown escape: keep both characters
            *idx -= 1;
            '\\'
        }
    }
}

fn is_blank_text_block_line(line: &str) -> bool {
    line.chars().all(|ch| matches!(ch, ' ' | '\t'))
}

fn normalize_text_block(raw: &str) -> String {
    let mut content = raw;
    if let Some(stripped) = content.strip_prefix('\n') {
        content = stripped;
    }
    if let Some(stripped) = content.strip_suffix('\n') {
        content = stripped;
    } else if let Some((without_last_line, last_line)) = content.rsplit_once('\n') {
        if is_blank_text_block_line(last_line) {
            content = without_last_line;
        }
    }

    if content.is_empty() {
        return String::new();
    }

    let Some(common_indent) = content
        .split('\n')
        .filter(|line| !is_blank_text_block_line(line))
        .map(|line| {
            line.chars()
                .take_while(|ch| matches!(ch, ' ' | '\t'))
                .count()
        })
        .min()
    else {
        return String::new();
    };

    let mut normalized = String::new();
    for (line_idx, line) in content.split('\n').enumerate() {
        if line_idx > 0 {
            normalized.push('\n');
        }
        if is_blank_text_block_line(line) {
            continue;
        }
        normalized.extend(line.chars().skip(common_indent));
    }

    normalized
}

fn scan_text_block_sigil(
    chars: &[char],
    idx: &mut usize,
    tokens: &mut Vec<Token>,
) -> Result<(), LexerError> {
    let start = *idx;
    if chars.get(*idx + 2) != Some(&'"')
        || chars.get(*idx + 3) != Some(&'"')
        || chars.get(*idx + 4) != Some(&'"')
    {
        return Err(LexerError::invalid_token(
            '~',
            Span::new(start, (start + 2).min(chars.len())),
        ));
    }

    *idx += 5;
    let mut literal = String::new();
    let mut terminated = false;

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
            return Err(LexerError::invalid_token('#', Span::new(*idx, *idx + 1)));
        }

        if chars[*idx] == '\\' {
            *idx += 1;
            literal.push(process_escape(chars, idx));
        } else {
            literal.push(chars[*idx]);
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
        normalize_text_block(&literal),
        Span::new(start, *idx),
    ));

    Ok(())
}

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
            if chars.get(temp_idx) == Some(&'\\') {
                temp_idx += 2; // skip escaped character
            } else {
                temp_idx += 1;
            }
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
            if peek == '\\' {
                temp_idx += 2; // skip escaped character
            } else {
                temp_idx += 1;
            }
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

                if chars[*idx] == '\\' {
                    *idx += 1;
                    literal.push(process_escape(chars, idx));
                } else {
                    literal.push(chars[*idx]);
                    *idx += 1;
                }
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

                if peek == '\\' {
                    *idx += 1;
                    literal.push(process_escape(chars, idx));
                } else {
                    literal.push(peek);
                    *idx += 1;
                }
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

            if chars[*idx] == '\\' {
                *idx += 1;
                literal.push(process_escape(chars, idx));
            } else {
                literal.push(chars[*idx]);
                *idx += 1;
            }
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

            if peek == '\\' {
                *idx += 1;
                literal.push(process_escape(chars, idx));
            } else {
                literal.push(peek);
                *idx += 1;
            }
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

    if sigil_kind == 't' {
        return scan_text_block_sigil(chars, idx, tokens);
    }

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

#[cfg(test)]
mod tests {
    use super::normalize_text_block;

    #[test]
    fn normalize_text_block_trims_framing_newlines_and_common_indent() {
        let normalized = normalize_text_block("\n    hello\n      world\n    done\n");
        assert_eq!(normalized, "hello\n  world\ndone");
    }

    #[test]
    fn normalize_text_block_ignores_blank_lines_when_computing_indent() {
        let normalized = normalize_text_block("\n        alpha\n\n          beta\n        gamma\n");
        assert_eq!(normalized, "alpha\n\n  beta\ngamma");
    }

    #[test]
    fn normalize_text_block_returns_empty_string_for_blank_block() {
        let normalized = normalize_text_block("\n    \n\t\n");
        assert_eq!(normalized, "");
    }
}
