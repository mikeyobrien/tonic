use super::types::{LexerError, Span, Token, TokenKind};
use super::{scan_tokens, LexerState};

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

#[derive(Debug, Clone)]
enum TextBlockFragment {
    Text {
        value: String,
        spans: Vec<Span>,
    },
    Expr {
        source: String,
        base_offset: usize,
        open_span: Span,
        close_span: Option<Span>,
    },
}

#[derive(Debug, Clone)]
enum TextBlockItem {
    Char { ch: char, span: Span },
    Expr { fragment_index: usize },
}

fn is_blank_text_block_item(item: &TextBlockItem) -> bool {
    matches!(item, TextBlockItem::Char { ch: ' ' | '\t', .. })
}

fn normalize_text_block_items(items: &[TextBlockItem]) -> Vec<TextBlockItem> {
    let mut content = items.to_vec();

    if matches!(content.first(), Some(TextBlockItem::Char { ch: '\n', .. })) {
        content.remove(0);
    }

    if matches!(content.last(), Some(TextBlockItem::Char { ch: '\n', .. })) {
        content.pop();
    } else if let Some(last_newline) = content
        .iter()
        .rposition(|item| matches!(item, TextBlockItem::Char { ch: '\n', .. }))
    {
        if content[last_newline + 1..]
            .iter()
            .all(is_blank_text_block_item)
        {
            content.truncate(last_newline);
        }
    }

    if content.is_empty() {
        return Vec::new();
    }

    let mut common_indent: Option<usize> = None;
    let mut line_start = 0usize;

    while line_start <= content.len() {
        let line_end = content[line_start..]
            .iter()
            .position(|item| matches!(item, TextBlockItem::Char { ch: '\n', .. }))
            .map(|idx| line_start + idx)
            .unwrap_or(content.len());
        let line = &content[line_start..line_end];

        if !line.iter().all(is_blank_text_block_item) {
            let indent = line
                .iter()
                .take_while(|item| is_blank_text_block_item(item))
                .count();
            common_indent = Some(common_indent.map_or(indent, |current| current.min(indent)));
        }

        if line_end == content.len() {
            break;
        }
        line_start = line_end + 1;
    }

    let Some(common_indent) = common_indent else {
        return Vec::new();
    };

    let mut normalized = Vec::new();
    line_start = 0;

    loop {
        let line_end = content[line_start..]
            .iter()
            .position(|item| matches!(item, TextBlockItem::Char { ch: '\n', .. }))
            .map(|idx| line_start + idx)
            .unwrap_or(content.len());
        let line = &content[line_start..line_end];

        if !line.iter().all(is_blank_text_block_item) {
            let mut remaining_indent = common_indent;
            for item in line {
                if remaining_indent > 0 && is_blank_text_block_item(item) {
                    remaining_indent -= 1;
                    continue;
                }
                normalized.push(item.clone());
            }
        }

        if line_end == content.len() {
            break;
        }

        normalized.push(content[line_end].clone());
        line_start = line_end + 1;
    }

    normalized
}

#[cfg(test)]
fn normalize_text_block(raw: &str) -> String {
    let items: Vec<TextBlockItem> = raw
        .chars()
        .enumerate()
        .map(|(idx, ch)| TextBlockItem::Char {
            ch,
            span: Span::new(idx, idx + 1),
        })
        .collect();

    normalized_text_block_string(&normalize_text_block_items(&items))
}

fn normalized_text_block_string(items: &[TextBlockItem]) -> String {
    let mut output = String::new();
    for item in items {
        if let TextBlockItem::Char { ch, .. } = item {
            output.push(*ch);
        }
    }
    output
}

fn flush_text_block_text_fragment(
    fragments: &mut Vec<TextBlockFragment>,
    current_value: &mut String,
    current_spans: &mut Vec<Span>,
) {
    if current_value.is_empty() {
        return;
    }

    fragments.push(TextBlockFragment::Text {
        value: std::mem::take(current_value),
        spans: std::mem::take(current_spans),
    });
}

fn scan_text_block_interpolation_source(chars: &[char], idx: &mut usize) -> TextBlockFragment {
    let open_start = *idx;
    *idx += 2;
    let base_offset = *idx;
    let mut source = String::new();
    let mut depth = 1usize;

    while *idx < chars.len() {
        if depth == 1
            && chars.get(*idx) == Some(&'"')
            && chars.get(*idx + 1) == Some(&'"')
            && chars.get(*idx + 2) == Some(&'"')
        {
            return TextBlockFragment::Expr {
                source,
                base_offset,
                open_span: Span::new(open_start, open_start + 2),
                close_span: None,
            };
        }

        if chars[*idx] == '#' && chars.get(*idx + 1) != Some(&'{') {
            while *idx < chars.len() {
                let ch = chars[*idx];
                source.push(ch);
                *idx += 1;
                if ch == '\n' {
                    break;
                }
            }
            continue;
        }

        if chars[*idx] == '"' {
            if chars.get(*idx + 1) == Some(&'"') && chars.get(*idx + 2) == Some(&'"') {
                source.push('"');
                source.push('"');
                source.push('"');
                *idx += 3;

                while *idx < chars.len() {
                    if chars.get(*idx) == Some(&'"')
                        && chars.get(*idx + 1) == Some(&'"')
                        && chars.get(*idx + 2) == Some(&'"')
                    {
                        source.push('"');
                        source.push('"');
                        source.push('"');
                        *idx += 3;
                        break;
                    }

                    if chars[*idx] == '\\' {
                        source.push(chars[*idx]);
                        *idx += 1;
                        if *idx < chars.len() {
                            source.push(chars[*idx]);
                            *idx += 1;
                        }
                    } else {
                        source.push(chars[*idx]);
                        *idx += 1;
                    }
                }
                continue;
            }

            source.push('"');
            *idx += 1;
            while *idx < chars.len() {
                let ch = chars[*idx];
                source.push(ch);
                *idx += 1;

                if ch == '\\' {
                    if *idx < chars.len() {
                        source.push(chars[*idx]);
                        *idx += 1;
                    }
                    continue;
                }

                if ch == '"' {
                    break;
                }
            }
            continue;
        }

        match chars[*idx] {
            '{' => {
                depth += 1;
                source.push('{');
                *idx += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let close_span = Span::new(*idx, *idx + 1);
                    *idx += 1;
                    return TextBlockFragment::Expr {
                        source,
                        base_offset,
                        open_span: Span::new(open_start, open_start + 2),
                        close_span: Some(close_span),
                    };
                }
                source.push('}');
                *idx += 1;
            }
            ch => {
                source.push(ch);
                *idx += 1;
            }
        }
    }

    TextBlockFragment::Expr {
        source,
        base_offset,
        open_span: Span::new(open_start, open_start + 2),
        close_span: None,
    }
}

fn shift_token(token: &Token, base_offset: usize) -> Token {
    let span = token.span();
    let shifted_span = Span::new(span.start() + base_offset, span.end() + base_offset);
    if token.lexeme().is_empty() {
        Token::simple(token.kind(), shifted_span)
    } else {
        Token::with_lexeme(token.kind(), token.lexeme().to_string(), shifted_span)
    }
}

fn shift_lexer_error(mut error: LexerError, base_offset: usize) -> LexerError {
    error.span = Span::new(
        error.span.start() + base_offset,
        error.span.end() + base_offset,
    );
    error
}

fn push_text_token(
    tokens: &mut Vec<Token>,
    kind: TokenKind,
    text: &mut String,
    spans: &mut Vec<Span>,
) {
    if text.is_empty() {
        return;
    }

    let span = Span::new(
        spans.first().map(|span| span.start()).unwrap_or(0),
        spans.last().map(|span| span.end()).unwrap_or(0),
    );
    tokens.push(Token::with_lexeme(kind, std::mem::take(text), span));
    spans.clear();
}

fn emit_text_block_tokens(
    fragments: &[TextBlockFragment],
    normalized_items: &[TextBlockItem],
    start: usize,
    end: usize,
    emit_string_end: bool,
    tokens: &mut Vec<Token>,
) -> Result<(), LexerError> {
    let has_interpolation = normalized_items
        .iter()
        .any(|item| matches!(item, TextBlockItem::Expr { .. }));

    if !has_interpolation {
        tokens.push(Token::with_lexeme(
            TokenKind::String,
            normalized_text_block_string(normalized_items),
            Span::new(start, end),
        ));
        return Ok(());
    }

    tokens.push(Token::simple(
        TokenKind::StringStart,
        Span::new(start, start + 5),
    ));

    let mut text = String::new();
    let mut spans = Vec::new();

    for item in normalized_items {
        match item {
            TextBlockItem::Char { ch, span } => {
                text.push(*ch);
                spans.push(*span);
            }
            TextBlockItem::Expr { fragment_index } => {
                push_text_token(tokens, TokenKind::StringPart, &mut text, &mut spans);

                let TextBlockFragment::Expr {
                    source,
                    base_offset,
                    open_span,
                    close_span,
                } = &fragments[*fragment_index]
                else {
                    unreachable!("expression placeholder should reference expr fragment");
                };

                tokens.push(Token::simple(TokenKind::InterpolationStart, *open_span));

                let mut expr_tokens =
                    scan_tokens(source).map_err(|error| shift_lexer_error(error, *base_offset))?;
                if matches!(
                    expr_tokens.last().map(|token| token.kind()),
                    Some(TokenKind::Eof)
                ) {
                    expr_tokens.pop();
                }
                tokens.extend(
                    expr_tokens
                        .iter()
                        .map(|token| shift_token(token, *base_offset)),
                );

                if let Some(close_span) = close_span {
                    tokens.push(Token::simple(TokenKind::InterpolationEnd, *close_span));
                }
            }
        }
    }

    push_text_token(tokens, TokenKind::StringPart, &mut text, &mut spans);
    if emit_string_end {
        tokens.push(Token::simple(TokenKind::StringEnd, Span::new(end - 3, end)));
    }
    Ok(())
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

    let mut fragments = Vec::new();
    let mut current_value = String::new();
    let mut current_spans = Vec::new();
    let mut terminated = false;

    while *idx < chars.len() {
        if chars.get(*idx) == Some(&'"')
            && chars.get(*idx + 1) == Some(&'"')
            && chars.get(*idx + 2) == Some(&'"')
        {
            flush_text_block_text_fragment(&mut fragments, &mut current_value, &mut current_spans);
            terminated = true;
            *idx += 3;
            break;
        }

        if chars.get(*idx) == Some(&'#') && chars.get(*idx + 1) == Some(&'{') {
            flush_text_block_text_fragment(&mut fragments, &mut current_value, &mut current_spans);
            let fragment = scan_text_block_interpolation_source(chars, idx);
            let interpolation_closed = matches!(
                fragment,
                TextBlockFragment::Expr {
                    close_span: Some(_),
                    ..
                }
            );
            fragments.push(fragment);

            if !interpolation_closed {
                let mut items = Vec::new();
                for (fragment_index, fragment) in fragments.iter().enumerate() {
                    match fragment {
                        TextBlockFragment::Text { value, spans } => {
                            for (ch, span) in value.chars().zip(spans.iter().copied()) {
                                items.push(TextBlockItem::Char { ch, span });
                            }
                        }
                        TextBlockFragment::Expr { .. } => {
                            items.push(TextBlockItem::Expr { fragment_index });
                        }
                    }
                }

                let normalized_items = normalize_text_block_items(&items);
                *idx = chars.len();
                return emit_text_block_tokens(
                    &fragments,
                    &normalized_items,
                    start,
                    *idx,
                    false,
                    tokens,
                );
            }

            continue;
        }

        if chars[*idx] == '\\' {
            let escape_start = *idx;
            *idx += 1;
            let value = process_escape(chars, idx);
            current_value.push(value);
            current_spans.push(Span::new(escape_start, *idx));
        } else {
            current_value.push(chars[*idx]);
            current_spans.push(Span::new(*idx, *idx + 1));
            *idx += 1;
        }
    }

    if !terminated {
        return Err(LexerError::unterminated_string(Span::new(
            start,
            chars.len(),
        )));
    }

    let mut items = Vec::new();
    for (fragment_index, fragment) in fragments.iter().enumerate() {
        match fragment {
            TextBlockFragment::Text { value, spans } => {
                for (ch, span) in value.chars().zip(spans.iter().copied()) {
                    items.push(TextBlockItem::Char { ch, span });
                }
            }
            TextBlockFragment::Expr { .. } => {
                items.push(TextBlockItem::Expr { fragment_index });
            }
        }
    }

    let normalized_items = normalize_text_block_items(&items);
    emit_text_block_tokens(&fragments, &normalized_items, start, *idx, true, tokens)
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
