use super::types::{LexerError, Span, Token, TokenKind};

/// Scan a numeric literal (decimal, hex, octal, binary, or float) in Normal state.
///
/// Called when the current character is an ASCII digit. Handles:
/// - `0x` / `0X` hex literals
/// - `0o` / `0O` octal literals
/// - `0b` / `0B` binary literals
/// - Decimal integers with optional underscore separators
/// - Float literals (`123.45`)
pub(super) fn scan_numeric(
    chars: &[char],
    idx: &mut usize,
    tokens: &mut Vec<Token>,
    start: usize,
    first_digit: char,
) -> Result<(), LexerError> {
    // Check for radix prefix: 0x, 0o, 0b
    if first_digit == '0' && *idx < chars.len() {
        match chars[*idx] {
            'x' | 'X' => {
                *idx += 1; // skip 'x'/'X'
                           // Error: no digits follow the prefix
                if *idx >= chars.len() || (!chars[*idx].is_ascii_hexdigit() && chars[*idx] != '_') {
                    return Err(LexerError::empty_numeric_literal(
                        "0x",
                        Span::new(start, *idx),
                    ));
                }
                // Error: separator at start
                if chars[*idx] == '_' {
                    return Err(LexerError::misplaced_numeric_separator(Span::new(
                        start,
                        *idx + 1,
                    )));
                }
                let digit_start = *idx;
                while *idx < chars.len() && (chars[*idx].is_ascii_hexdigit() || chars[*idx] == '_')
                {
                    *idx += 1;
                }
                // Error: separator at end
                if chars[*idx - 1] == '_' {
                    return Err(LexerError::misplaced_numeric_separator(Span::new(
                        start, *idx,
                    )));
                }
                let digits: String = chars[digit_start..*idx]
                    .iter()
                    .filter(|c| **c != '_')
                    .collect();
                let int_value = i64::from_str_radix(&digits, 16)
                    .map_err(|_| LexerError::empty_numeric_literal("0x", Span::new(start, *idx)))?;
                tokens.push(Token::with_lexeme(
                    TokenKind::Integer,
                    int_value.to_string(),
                    Span::new(start, *idx),
                ));
                return Ok(());
            }
            'o' | 'O' => {
                *idx += 1; // skip 'o'/'O'
                           // Error: no digits follow the prefix
                if *idx >= chars.len()
                    || (!('0'..='7').contains(&chars[*idx]) && chars[*idx] != '_')
                {
                    // Check for invalid digit (e.g. 0o8)
                    if *idx < chars.len() && chars[*idx].is_ascii_digit() {
                        return Err(LexerError::invalid_digit_for_base(
                            chars[*idx],
                            "octal",
                            Span::new(start, *idx + 1),
                        ));
                    }
                    return Err(LexerError::empty_numeric_literal(
                        "0o",
                        Span::new(start, *idx),
                    ));
                }
                // Error: separator at start
                if chars[*idx] == '_' {
                    return Err(LexerError::misplaced_numeric_separator(Span::new(
                        start,
                        *idx + 1,
                    )));
                }
                let digit_start = *idx;
                while *idx < chars.len() {
                    if chars[*idx] == '_' || (chars[*idx].is_ascii_digit() && chars[*idx] <= '7') {
                        *idx += 1;
                    } else if chars[*idx].is_ascii_digit() {
                        // digit 8 or 9 in octal literal
                        return Err(LexerError::invalid_digit_for_base(
                            chars[*idx],
                            "octal",
                            Span::new(start, *idx + 1),
                        ));
                    } else {
                        break;
                    }
                }
                // Error: separator at end
                if chars[*idx - 1] == '_' {
                    return Err(LexerError::misplaced_numeric_separator(Span::new(
                        start, *idx,
                    )));
                }
                let digits: String = chars[digit_start..*idx]
                    .iter()
                    .filter(|c| **c != '_')
                    .collect();
                let int_value = i64::from_str_radix(&digits, 8)
                    .map_err(|_| LexerError::empty_numeric_literal("0o", Span::new(start, *idx)))?;
                tokens.push(Token::with_lexeme(
                    TokenKind::Integer,
                    int_value.to_string(),
                    Span::new(start, *idx),
                ));
                return Ok(());
            }
            'b' | 'B' => {
                *idx += 1; // skip 'b'/'B'
                           // Error: no digits follow the prefix
                if *idx >= chars.len()
                    || (chars[*idx] != '0' && chars[*idx] != '1' && chars[*idx] != '_')
                {
                    // Check for invalid digit (e.g. 0b2)
                    if *idx < chars.len() && chars[*idx].is_ascii_digit() {
                        return Err(LexerError::invalid_digit_for_base(
                            chars[*idx],
                            "binary",
                            Span::new(start, *idx + 1),
                        ));
                    }
                    return Err(LexerError::empty_numeric_literal(
                        "0b",
                        Span::new(start, *idx),
                    ));
                }
                // Error: separator at start
                if chars[*idx] == '_' {
                    return Err(LexerError::misplaced_numeric_separator(Span::new(
                        start,
                        *idx + 1,
                    )));
                }
                let digit_start = *idx;
                while *idx < chars.len() {
                    if chars[*idx] == '_' || matches!(chars[*idx], '0' | '1') {
                        *idx += 1;
                    } else if chars[*idx].is_ascii_digit() {
                        // digit 2-9 in binary literal
                        return Err(LexerError::invalid_digit_for_base(
                            chars[*idx],
                            "binary",
                            Span::new(start, *idx + 1),
                        ));
                    } else {
                        break;
                    }
                }
                // Error: separator at end
                if chars[*idx - 1] == '_' {
                    return Err(LexerError::misplaced_numeric_separator(Span::new(
                        start, *idx,
                    )));
                }
                let digits: String = chars[digit_start..*idx]
                    .iter()
                    .filter(|c| **c != '_')
                    .collect();
                let int_value = i64::from_str_radix(&digits, 2)
                    .map_err(|_| LexerError::empty_numeric_literal("0b", Span::new(start, *idx)))?;
                tokens.push(Token::with_lexeme(
                    TokenKind::Integer,
                    int_value.to_string(),
                    Span::new(start, *idx),
                ));
                return Ok(());
            }
            _ => {}
        }
    }

    // Decimal integer: consume digits and underscores
    while *idx < chars.len() && (chars[*idx].is_ascii_digit() || chars[*idx] == '_') {
        *idx += 1;
    }

    // Error: separator at end of integer part
    if chars[*idx - 1] == '_' {
        return Err(LexerError::misplaced_numeric_separator(Span::new(
            start, *idx,
        )));
    }

    let mut kind = TokenKind::Integer;
    if *idx + 1 < chars.len() && chars[*idx] == '.' && chars[*idx + 1].is_ascii_digit() {
        kind = TokenKind::Float;
        *idx += 1;

        while *idx < chars.len() && (chars[*idx].is_ascii_digit() || chars[*idx] == '_') {
            *idx += 1;
        }

        // Error: separator at end of fractional part
        if chars[*idx - 1] == '_' {
            return Err(LexerError::misplaced_numeric_separator(Span::new(
                start, *idx,
            )));
        }
    }

    // Strip underscores from the lexeme
    let lexeme: String = chars[start..*idx].iter().filter(|c| **c != '_').collect();
    tokens.push(Token::with_lexeme(kind, lexeme, Span::new(start, *idx)));
    Ok(())
}
