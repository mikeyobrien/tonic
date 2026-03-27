use crate::lexer::{scan_tokens_with_comments, Comment, Token, TokenKind};

/// Format a Tonic source string using a token-driven approach.
///
/// The lexer is used to scan the source into tokens plus a comment sidecar,
/// then the token stream is formatted with 2-space indentation and comments
/// are merged back onto logical lines.
///
/// If lexing fails (malformed source), the original normalized source is
/// returned unchanged to avoid corrupting code with syntax errors.
pub(super) fn format_source_inner(source: &str) -> String {
    let normalized = normalize_newlines(source);

    match scan_tokens_with_comments(&normalized) {
        Ok((tokens, comments)) => format_tokens(&normalized, &tokens, &comments),
        Err(_) => normalized,
    }
}

// ---------------------------------------------------------------------------
// Two-pass token-based formatter
//
// Pass 1: Segment tokens into logical lines.
//   A new logical line starts after: Do, End, Else, Rescue, Catch, After,
//   Arrow (->), Semicolon, and at the top level between Pipe operators.
//
// Pass 2: Apply indentation to each logical line based on its content.
// ---------------------------------------------------------------------------

/// A logical line is a list of (TokenKind, text) pairs that will be emitted
/// on a single output line.
#[derive(Debug, Clone)]
pub(super) struct LogicalLine {
    /// Token kinds for classification
    kinds: Vec<TokenKind>,
    /// The rendered text of the line (without indentation)
    pub(super) text: String,
    /// Whether this line has a blank line gap before it (from source)
    pub(super) blank_before: bool,
    source_line_start: Option<usize>,
    source_line_end: Option<usize>,
}

impl LogicalLine {
    fn new() -> Self {
        Self {
            kinds: Vec::new(),
            text: String::new(),
            blank_before: false,
            source_line_start: None,
            source_line_end: None,
        }
    }

    fn comment(comment: &Comment) -> Self {
        Self {
            kinds: Vec::new(),
            text: comment.text().to_string(),
            blank_before: comment.blank_lines_before() > 0,
            source_line_start: Some(comment.line()),
            source_line_end: Some(comment.line()),
        }
    }

    fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    fn first_kind(&self) -> Option<TokenKind> {
        self.kinds.first().copied()
    }

    fn last_kind(&self) -> Option<TokenKind> {
        self.kinds.last().copied()
    }

    fn note_source_line(&mut self, source_line: usize) {
        if self.source_line_start.is_none() {
            self.source_line_start = Some(source_line);
        }
        self.source_line_end = Some(source_line);
    }

    fn contains_source_line(&self, source_line: usize) -> bool {
        matches!(
            (self.source_line_start, self.source_line_end),
            (Some(start), Some(end)) if start <= source_line && source_line <= end
        )
    }

    fn push(&mut self, kind: TokenKind, text: &str) {
        let space = if self.text.is_empty() {
            false
        } else {
            space_between(self.last_kind().unwrap(), kind, &self.text, text)
        };

        if space {
            self.text.push(' ');
        }
        self.text.push_str(text);
        self.kinds.push(kind);
    }

    fn append_trailing_comment(&mut self, comment: &Comment) {
        if !self.text.is_empty() {
            self.text.push(' ');
        }
        self.text.push_str(comment.text());
    }
}

/// Determine whether a space is needed between two adjacent tokens.
fn space_between(prev: TokenKind, curr: TokenKind, prev_text: &str, curr_text: &str) -> bool {
    use TokenKind::*;

    match curr {
        RParen | RBracket | RBrace | Comma | Dot | Semicolon | Question | Eof => return false,
        StringPart | InterpolationEnd | StringEnd => return false,
        _ => {}
    }

    match prev {
        LBracket | Bang | At | Ampersand => return false,
        StringStart | StringPart | InterpolationStart => return false,
        Dot => return false,
        _ => {}
    }

    if prev == Percent {
        return false;
    }

    if curr == LParen && matches!(prev, Ident | RParen | RBracket | RBrace) {
        return false;
    }

    if curr == LBrace && prev == Ident {
        return false;
    }

    if curr == Colon {
        return false;
    }

    if prev == LParen || prev == LBrace {
        return false;
    }

    if curr == StringStart {
        return !matches!(prev, At | Ampersand | LParen | LBracket | Comma);
    }

    _ = prev_text;
    _ = curr_text;

    true
}

#[allow(unused_assignments)]
pub(super) fn build_logical_lines(source: &str, tokens: &[Token]) -> Vec<LogicalLine> {
    let line_for_offset = compute_line_map(source);

    let mut lines: Vec<LogicalLine> = Vec::new();
    let mut current = LogicalLine::new();
    let mut interp_depth: usize = 0;
    let mut paren_depth: usize = 0;
    let mut bracket_depth: usize = 0;
    let mut brace_depth: usize = 0;
    let mut prev_line_num: Option<usize> = None;

    let blank_before_line = compute_blank_before_map(source);

    macro_rules! flush {
        () => {
            if !current.is_empty() {
                lines.push(current.clone());
                current = LogicalLine::new();
                prev_line_num = None;
            }
        };
    }

    let n = tokens.len();
    let mut i = 0;

    while i < n {
        let token = &tokens[i];
        let kind = token.kind();

        if kind == TokenKind::Eof {
            break;
        }

        let text = token_text(token, source);
        let tok_line = line_for_offset
            .get(token.span().start())
            .copied()
            .unwrap_or(0);

        if interp_depth > 0 {
            current.note_source_line(tok_line);
            match kind {
                TokenKind::InterpolationEnd => {
                    interp_depth -= 1;
                    current.push(kind, "}");
                }
                TokenKind::StringEnd => {
                    current.push(kind, "\"");
                }
                _ => {
                    current.push(kind, text);
                }
            }
            i += 1;
            continue;
        }

        let inline = paren_depth > 0 || bracket_depth > 0 || brace_depth > 0;

        match kind {
            TokenKind::StringStart => {
                current.note_source_line(tok_line);
                current.push(kind, "\"");
            }
            TokenKind::StringPart => {
                current.note_source_line(tok_line);
                current.push(kind, text);
            }
            TokenKind::InterpolationStart => {
                interp_depth += 1;
                current.note_source_line(tok_line);
                current.push(kind, "#{");
            }
            TokenKind::StringEnd => {
                current.note_source_line(tok_line);
                current.push(kind, "\"");
            }
            TokenKind::String => {
                current.note_source_line(tok_line);
                current.push(kind, text);
            }
            TokenKind::LParen => {
                paren_depth += 1;
                current.note_source_line(tok_line);
                current.push(kind, "(");
            }
            TokenKind::RParen => {
                paren_depth = paren_depth.saturating_sub(1);
                current.note_source_line(tok_line);
                current.push(kind, ")");
            }
            TokenKind::LBracket => {
                bracket_depth += 1;
                current.note_source_line(tok_line);
                current.push(kind, "[");
            }
            TokenKind::RBracket => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.note_source_line(tok_line);
                current.push(kind, "]");
            }
            TokenKind::LBrace => {
                brace_depth += 1;
                current.note_source_line(tok_line);
                current.push(kind, "{");
            }
            TokenKind::RBrace => {
                brace_depth = brace_depth.saturating_sub(1);
                current.note_source_line(tok_line);
                current.push(kind, "}");
            }
            TokenKind::Do => {
                current.note_source_line(tok_line);
                current.push(kind, "do");
                flush!();
            }
            TokenKind::End => {
                flush!();
                current.note_source_line(tok_line);
                current.push(kind, "end");
                flush!();
            }
            TokenKind::Else | TokenKind::Rescue | TokenKind::Catch | TokenKind::After => {
                flush!();
                current.note_source_line(tok_line);
                current.push(kind, text);
            }
            TokenKind::Arrow => {
                current.note_source_line(tok_line);
                current.push(kind, "->");
                let next_tok = tokens.get(i + 1);
                let break_after = match next_tok {
                    None => true,
                    Some(next) => {
                        let next_kind = next.kind();
                        if matches!(
                            next_kind,
                            TokenKind::Do
                                | TokenKind::End
                                | TokenKind::Else
                                | TokenKind::Rescue
                                | TokenKind::Catch
                                | TokenKind::After
                                | TokenKind::Eof
                        ) {
                            true
                        } else {
                            let next_line = line_for_offset
                                .get(next.span().start())
                                .copied()
                                .unwrap_or(0);
                            next_line > tok_line
                        }
                    }
                };
                if break_after {
                    flush!();
                }
            }
            TokenKind::PipeGt if !inline => {
                flush!();
                current.note_source_line(tok_line);
                current.push(kind, "|>");
            }
            TokenKind::Semicolon => {
                flush!();
            }
            _ => {
                if !inline {
                    if let Some(prev_ln) = prev_line_num {
                        if tok_line > prev_ln {
                            flush!();
                            if blank_before_line.get(tok_line).copied().unwrap_or(false) {
                                current.blank_before = true;
                            }
                        }
                    }
                }
                current.note_source_line(tok_line);
                current.push(kind, text);
            }
        }

        prev_line_num = Some(tok_line);
        i += 1;
    }

    flush!();
    lines
}

fn merge_comments(lines: Vec<LogicalLine>, comments: &[Comment]) -> Vec<LogicalLine> {
    if comments.is_empty() {
        return lines;
    }

    let mut merged = Vec::with_capacity(lines.len() + comments.len());
    let mut comment_idx = 0usize;

    for mut line in lines {
        while let Some(comment) = comments.get(comment_idx) {
            if comment.has_code_before() {
                break;
            }
            let Some(line_start) = line.source_line_start else {
                break;
            };
            if comment.line() < line_start {
                merged.push(LogicalLine::comment(comment));
                comment_idx += 1;
                continue;
            }
            break;
        }

        while let Some(comment) = comments.get(comment_idx) {
            if !comment.has_code_before() || !line.contains_source_line(comment.line()) {
                break;
            }
            line.append_trailing_comment(comment);
            comment_idx += 1;
        }

        merged.push(line);
    }

    while let Some(comment) = comments.get(comment_idx) {
        if comment.has_code_before() {
            if let Some(last_line) = merged.last_mut() {
                last_line.append_trailing_comment(comment);
            } else {
                merged.push(LogicalLine::comment(comment));
            }
        } else {
            merged.push(LogicalLine::comment(comment));
        }
        comment_idx += 1;
    }

    merged
}

fn compute_line_map(source: &str) -> Vec<usize> {
    let mut map = Vec::with_capacity(source.len() + 1);
    let mut line = 0usize;
    for ch in source.chars() {
        map.push(line);
        if ch == '\n' {
            line += 1;
        }
    }
    map.push(line);
    map
}

fn compute_blank_before_map(source: &str) -> Vec<bool> {
    let source_lines: Vec<&str> = source.lines().collect();
    let mut result = vec![false; source_lines.len() + 1];
    let mut prev_was_blank = false;
    for (i, line) in source_lines.iter().enumerate() {
        if line.trim().is_empty() {
            prev_was_blank = true;
        } else {
            result[i] = prev_was_blank;
            prev_was_blank = false;
        }
    }
    result
}

pub(super) fn apply_indentation(lines: &[LogicalLine]) -> String {
    let mut output = String::new();
    let mut indent: i32 = 0;
    let mut in_branch_body = false;

    for line in lines {
        if line.blank_before {
            output.push('\n');
        }

        let first = line.first_kind();
        let last = line.last_kind();
        let ends_arrow = last == Some(TokenKind::Arrow);
        let ends_do = last == Some(TokenKind::Do);
        let is_end = first == Some(TokenKind::End);
        let is_block_reopen = matches!(
            first,
            Some(TokenKind::Else | TokenKind::Rescue | TokenKind::Catch | TokenKind::After)
        );
        let is_branch_head = ends_arrow;

        if is_block_reopen || is_end {
            if in_branch_body {
                indent = indent.saturating_sub(1);
                in_branch_body = false;
            }
            indent = indent.saturating_sub(1);
        } else if is_branch_head && in_branch_body {
            indent = indent.saturating_sub(1);
            in_branch_body = false;
        }

        if indent < 0 {
            indent = 0;
        }

        let indent_str = "  ".repeat(indent as usize);
        output.push_str(&indent_str);
        output.push_str(&line.text);
        output.push('\n');

        if is_block_reopen {
            indent += 1;
            in_branch_body = false;
        }

        if ends_do {
            indent += 1;
            in_branch_body = false;
        } else if is_branch_head {
            indent += 1;
            in_branch_body = true;
        }

        if indent < 0 {
            indent = 0;
        }
    }

    output
}

pub(super) fn format_tokens(source: &str, tokens: &[Token], comments: &[Comment]) -> String {
    let lines = build_logical_lines(source, tokens);
    let merged = merge_comments(lines, comments);
    let mut output = apply_indentation(&merged);

    while output.ends_with("\n\n") {
        output.pop();
    }
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    output
}

fn token_text<'a>(token: &Token, source: &'a str) -> &'a str {
    let span = token.span();
    &source[span.start()..span.end()]
}

fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}
