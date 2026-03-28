mod numeric;
mod string_scan;
pub(crate) mod types;

pub use types::{Comment, LexerError, Span, Token, TokenKind};

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_extended;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexerState {
    Normal,
    String {
        is_heredoc: bool,
        brace_depth: usize,
    },
}

pub fn scan_tokens(source: &str) -> Result<Vec<Token>, LexerError> {
    scan_tokens_with_comments(source).map(|(tokens, _comments)| tokens)
}

pub fn scan_tokens_with_comments(source: &str) -> Result<(Vec<Token>, Vec<Comment>), LexerError> {
    let chars: Vec<char> = source.chars().collect();
    let (line_for_offset, line_start_offsets, blank_lines_before) = compute_source_layout(&chars);

    let mut tokens = Vec::new();
    let mut comments = Vec::new();
    let mut idx = 0;

    let mut state_stack = vec![LexerState::Normal];
    let mut current_brace_depth: usize = 0;

    while idx < chars.len() {
        let current = chars[idx];

        let state = *state_stack.last().unwrap();

        match state {
            LexerState::Normal => {
                if current.is_whitespace() {
                    idx += 1;
                    continue;
                }

                if current == '#' {
                    let start = idx;
                    idx += 1;
                    while idx < chars.len() && chars[idx] != '\n' {
                        idx += 1;
                    }

                    let line = line_for_offset.get(start).copied().unwrap_or(0);
                    let line_start = line_start_offsets.get(line).copied().unwrap_or(0);
                    let column = start.saturating_sub(line_start);
                    let text: String = chars[start..idx].iter().collect();
                    let has_code_before = chars[line_start..start]
                        .iter()
                        .any(|value| !value.is_whitespace());

                    comments.push(Comment::new(
                        text,
                        Span::new(start, idx),
                        line,
                        column,
                        blank_lines_before.get(line).copied().unwrap_or(0),
                        has_code_before,
                    ));
                    continue;
                }

                match current {
                    '(' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::LParen, Span::new(start, idx)));
                    }
                    ')' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::RParen, Span::new(start, idx)));
                    }
                    '{' => {
                        let start = idx;
                        idx += 1;
                        current_brace_depth += 1;
                        tokens.push(Token::simple(TokenKind::LBrace, Span::new(start, idx)));
                    }
                    '}' => {
                        let start = idx;
                        idx += 1;
                        current_brace_depth = current_brace_depth.saturating_sub(1);

                        if state_stack.len() > 1 {
                            if let LexerState::String { brace_depth, .. } =
                                state_stack[state_stack.len() - 2]
                            {
                                if current_brace_depth == brace_depth {
                                    state_stack.pop();
                                    tokens.push(Token::simple(
                                        TokenKind::InterpolationEnd,
                                        Span::new(start, idx),
                                    ));
                                    continue;
                                }
                            }
                        }
                        tokens.push(Token::simple(TokenKind::RBrace, Span::new(start, idx)));
                    }
                    '[' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::LBracket, Span::new(start, idx)));
                    }
                    ']' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::RBracket, Span::new(start, idx)));
                    }
                    '%' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Percent, Span::new(start, idx)));
                    }
                    '@' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::At, Span::new(start, idx)));
                    }
                    ',' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Comma, Span::new(start, idx)));
                    }
                    ';' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Semicolon, Span::new(start, idx)));
                    }
                    '.' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'.') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::DotDot, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Dot, Span::new(start, idx)));
                        }
                    }
                    '^' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'^') && chars.get(idx + 2) == Some(&'^') {
                            idx += 3;
                            tokens.push(Token::simple(
                                TokenKind::CaretCaretCaret,
                                Span::new(start, idx),
                            ));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Caret, Span::new(start, idx)));
                        }
                    }
                    '+' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'+') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::PlusPlus, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Plus, Span::new(start, idx)));
                        }
                    }
                    '-' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::Arrow, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'-') {
                            idx += 2;
                            tokens
                                .push(Token::simple(TokenKind::MinusMinus, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Minus, Span::new(start, idx)));
                        }
                    }
                    '*' => {
                        let start = idx;
                        idx += 1;
                        tokens.push(Token::simple(TokenKind::Star, Span::new(start, idx)));
                    }
                    '/' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'/') {
                            idx += 2;
                            tokens
                                .push(Token::simple(TokenKind::SlashSlash, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Slash, Span::new(start, idx)));
                        }
                    }
                    '\\' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'\\') {
                            idx += 2;
                            tokens.push(Token::simple(
                                TokenKind::BackslashBackslash,
                                Span::new(start, idx),
                            ));
                        } else {
                            return Err(LexerError::invalid_token(
                                '\\',
                                Span::new(start, start + 1),
                            ));
                        }
                    }
                    '=' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::FatArrow, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=')
                            && chars.get(idx + 2) == Some(&'=')
                        {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::StrictEq, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::EqEq, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::MatchEq, Span::new(start, idx)));
                        }
                    }
                    '!' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'=') && chars.get(idx + 2) == Some(&'=') {
                            idx += 3;
                            tokens.push(Token::simple(
                                TokenKind::StrictBangEq,
                                Span::new(start, idx),
                            ));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::BangEq, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Bang, Span::new(start, idx)));
                        }
                    }
                    '<' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'<') && chars.get(idx + 2) == Some(&'<') {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::LtLtLt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'<') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::LtLt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::LtEq, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'-') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::LeftArrow, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens
                                .push(Token::simple(TokenKind::LessGreater, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Lt, Span::new(start, idx)));
                        }
                    }
                    '>' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') && chars.get(idx + 2) == Some(&'>') {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::GtGtGt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::GtGt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'=') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::GtEq, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Gt, Span::new(start, idx)));
                        }
                    }
                    '?' => {
                        let start = idx;
                        idx += 1;
                        if idx < chars.len()
                            && chars[idx] != ' '
                            && chars[idx] != '\n'
                            && chars[idx] != '\t'
                            && chars[idx] != ')'
                            && chars[idx] != ','
                            && chars[idx] != ']'
                            && chars[idx] != '}'
                        {
                            let char_value: u32;
                            if chars[idx] == '\\' && idx + 1 < chars.len() {
                                idx += 1;
                                char_value = match chars[idx] {
                                    'n' => 10,
                                    't' => 9,
                                    'r' => 13,
                                    's' => 32,
                                    '\\' => 92,
                                    '"' => 34,
                                    '\'' => 39,
                                    '0' => 0,
                                    other => other as u32,
                                };
                                idx += 1;
                            } else {
                                char_value = chars[idx] as u32;
                                idx += 1;
                            }
                            let lexeme = char_value.to_string();
                            tokens.push(Token::with_lexeme(
                                TokenKind::Integer,
                                lexeme,
                                Span::new(start, idx),
                            ));
                        } else {
                            tokens.push(Token::simple(TokenKind::Question, Span::new(start, idx)));
                        }
                    }
                    '|' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'>') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::PipeGt, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'|')
                            && chars.get(idx + 2) == Some(&'|')
                        {
                            idx += 3;
                            tokens.push(Token::simple(
                                TokenKind::PipePipePipe,
                                Span::new(start, idx),
                            ));
                        } else if chars.get(idx + 1) == Some(&'|') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::OrOr, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Pipe, Span::new(start, idx)));
                        }
                    }
                    '&' => {
                        let start = idx;
                        if chars.get(idx + 1) == Some(&'&') && chars.get(idx + 2) == Some(&'&') {
                            idx += 3;
                            tokens.push(Token::simple(TokenKind::AmpAmpAmp, Span::new(start, idx)));
                        } else if chars.get(idx + 1) == Some(&'&') {
                            idx += 2;
                            tokens.push(Token::simple(TokenKind::AndAnd, Span::new(start, idx)));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Ampersand, Span::new(start, idx)));
                        }
                    }
                    ':' => {
                        let start = idx;

                        if chars.get(idx + 1).is_some_and(|next| is_ident_start(*next)) {
                            idx += 1;
                            let atom_start = idx;
                            idx += 1;

                            idx = scan_identifier_tail(&chars, idx, IdentifierScanMode::Atom);

                            let lexeme: String = chars[atom_start..idx].iter().collect();
                            tokens.push(Token::with_lexeme(
                                TokenKind::Atom,
                                lexeme,
                                Span::new(start, idx),
                            ));
                        } else {
                            idx += 1;
                            tokens.push(Token::simple(TokenKind::Colon, Span::new(start, idx)));
                        }
                    }
                    '~' => {
                        string_scan::scan_sigil(&chars, &mut idx, &mut tokens)?;
                    }
                    '"' => {
                        string_scan::scan_string_literal(
                            &chars,
                            &mut idx,
                            &mut tokens,
                            &mut state_stack,
                            current_brace_depth,
                        )?;
                    }
                    value if value.is_ascii_digit() => {
                        let start = idx;
                        idx += 1;
                        numeric::scan_numeric(&chars, &mut idx, &mut tokens, start, value)?;
                    }
                    value if is_ident_start(value) => {
                        let start = idx;
                        idx += 1;

                        idx = scan_identifier_tail(&chars, idx, IdentifierScanMode::Ident);

                        let lexeme: String = chars[start..idx].iter().collect();
                        let kind = keyword_kind(&lexeme).unwrap_or(TokenKind::Ident);
                        tokens.push(Token::with_lexeme(kind, lexeme, Span::new(start, idx)));
                    }
                    unexpected => {
                        return Err(LexerError::invalid_token(
                            unexpected,
                            Span::new(idx, idx + 1),
                        ));
                    }
                }
            }
            LexerState::String {
                is_heredoc,
                brace_depth: _,
            } => {
                string_scan::scan_string_content(
                    &chars,
                    &mut idx,
                    &mut tokens,
                    &mut state_stack,
                    &mut current_brace_depth,
                    is_heredoc,
                )?;
            }
        }
    }

    tokens.push(Token::simple(
        TokenKind::Eof,
        Span::new(chars.len(), chars.len()),
    ));
    Ok((tokens, comments))
}

fn compute_source_layout(chars: &[char]) -> (Vec<usize>, Vec<usize>, Vec<usize>) {
    let mut line_for_offset = Vec::with_capacity(chars.len() + 1);
    let mut line_start_offsets = vec![0];
    let mut line = 0usize;

    for (idx, ch) in chars.iter().enumerate() {
        line_for_offset.push(line);
        if *ch == '\n' {
            line += 1;
            line_start_offsets.push(idx + 1);
        }
    }
    line_for_offset.push(line);

    let mut blank_lines_before = vec![0; line_start_offsets.len()];
    let mut blank_run = 0usize;

    for line_idx in 0..line_start_offsets.len() {
        let start = line_start_offsets[line_idx];
        let end = if line_idx + 1 < line_start_offsets.len() {
            line_start_offsets[line_idx + 1].saturating_sub(1)
        } else {
            chars.len()
        };
        let is_blank = chars[start..end].iter().all(|value| value.is_whitespace());
        if is_blank {
            blank_run += 1;
        } else {
            blank_lines_before[line_idx] = blank_run;
            blank_run = 0;
        }
    }

    (line_for_offset, line_start_offsets, blank_lines_before)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IdentifierScanMode {
    Ident,
    Atom,
}

fn scan_identifier_tail(chars: &[char], mut idx: usize, mode: IdentifierScanMode) -> usize {
    while idx < chars.len() && is_ident_continue(chars[idx]) {
        idx += 1;
    }

    if chars.get(idx) == Some(&'?') && trailing_question_belongs_to_identifier(chars, idx, mode) {
        idx += 1;
    }

    idx
}

fn trailing_question_belongs_to_identifier(
    chars: &[char],
    question_idx: usize,
    mode: IdentifierScanMode,
) -> bool {
    let next = chars.get(question_idx + 1).copied();

    match mode {
        IdentifierScanMode::Ident => matches!(next, Some('(' | '/' | ':')),
        IdentifierScanMode::Atom => next.is_none_or(atom_question_boundary),
    }
}

fn atom_question_boundary(value: char) -> bool {
    value.is_whitespace() || matches!(value, ',' | ')' | ']' | '}' | ':')
}

fn keyword_kind(lexeme: &str) -> Option<TokenKind> {
    match lexeme {
        "defmodule" => Some(TokenKind::Defmodule),
        "def" => Some(TokenKind::Def),
        "defp" => Some(TokenKind::Defp),
        "do" => Some(TokenKind::Do),
        "end" => Some(TokenKind::End),
        "if" => Some(TokenKind::If),
        "unless" => Some(TokenKind::Unless),
        "case" => Some(TokenKind::Case),
        "cond" => Some(TokenKind::Cond),
        "with" => Some(TokenKind::With),
        "for" => Some(TokenKind::For),
        "fn" => Some(TokenKind::Fn),
        "else" => Some(TokenKind::Else),
        "try" => Some(TokenKind::Try),
        "rescue" => Some(TokenKind::Rescue),
        "catch" => Some(TokenKind::Catch),
        "after" => Some(TokenKind::After),
        "raise" => Some(TokenKind::Raise),
        "true" => Some(TokenKind::True),
        "false" => Some(TokenKind::False),
        "nil" => Some(TokenKind::Nil),
        "and" => Some(TokenKind::And),
        "or" => Some(TokenKind::Or),
        "not" => Some(TokenKind::Not),
        "in" => Some(TokenKind::In),
        "when" => Some(TokenKind::When),
        _ => None,
    }
}

fn is_ident_start(value: char) -> bool {
    value == '_' || value.is_ascii_alphabetic()
}

fn is_ident_continue(value: char) -> bool {
    is_ident_start(value) || value.is_ascii_digit()
}
