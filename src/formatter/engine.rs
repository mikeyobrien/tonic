use crate::lexer::{scan_tokens, Token, TokenKind};

/// Format a Tonic source string using a token-driven approach.
///
/// The lexer is used to scan the source into tokens, then the token
/// stream is formatted with 2-space indentation.
///
/// Known limitation: comments (`# ...`) are stripped by the lexer and
/// not preserved in the formatted output. This is a known alpha-stage
/// limitation; comment preservation requires a comment-aware token stream.
///
/// If lexing fails (malformed source), the original normalized source is
/// returned unchanged to avoid corrupting code with syntax errors.
pub(super) fn format_source_inner(source: &str) -> String {
    let normalized = normalize_newlines(source);

    match scan_tokens(&normalized) {
        Ok(tokens) => format_tokens(&normalized, &tokens),
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
}

impl LogicalLine {
    fn new() -> Self {
        Self {
            kinds: Vec::new(),
            text: String::new(),
            blank_before: false,
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
}

/// Determine whether a space is needed between two adjacent tokens.
fn space_between(prev: TokenKind, curr: TokenKind, prev_text: &str, curr_text: &str) -> bool {
    use TokenKind::*;

    // Never space before these punctuation tokens
    match curr {
        RParen | RBracket | RBrace | Comma | Dot | Semicolon | Question | Eof => return false,
        StringPart | InterpolationEnd | StringEnd => return false,
        _ => {}
    }

    // Never space after these
    match prev {
        LBracket | Bang | At | Ampersand => return false,
        StringStart | StringPart | InterpolationStart => return false,
        Dot => return false,
        _ => {}
    }

    // No space: %Module or %{ (struct/map literal)
    if prev == Percent {
        return false;
    }

    // No space before ( in function call: ident(
    if curr == LParen && matches!(prev, Ident | RParen | RBracket | RBrace) {
        return false;
    }

    // No space for { after ident when it's a struct field: Module{
    // (Percent already handled above; Ident{ only occurs in %Module{...})
    if curr == LBrace && prev == Ident {
        return false;
    }

    // Colon in key: value (label) → no space before, space after handled by comma
    if curr == Colon {
        return false;
    }

    // No space after opening paren or brace
    if prev == LParen || prev == LBrace {
        return false;
    }

    // String continuations
    if curr == StringStart {
        return !matches!(prev, At | Ampersand | LParen | LBracket | Comma);
    }

    _ = prev_text;
    _ = curr_text;

    true
}

/// Build a sequence of logical lines from the token stream.
///
/// Strategy: use a combination of source-line position and structural tokens
/// to determine line breaks. Two adjacent tokens on different source lines
/// (at nesting depth 0) trigger a line break, UNLESS the token is a `do`
/// (which always attaches to the line above) or `end`/`else`/etc (which
/// are handled structurally).
///
/// Structural tokens that always create line breaks regardless of nesting:
/// - After `do` (opens a block)
/// - Before/after `end`
/// - Before/after `else`/`rescue`/`catch`/`after`
/// - After `->` (opens a branch body)
///
/// When tokens are on the same source line (or inside parens/brackets),
/// they are kept together on the same logical line.
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

        // Inside string interpolations: accumulate tokens inline (no line breaks)
        if interp_depth > 0 {
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
            // String literals: accumulate inline
            TokenKind::StringStart => {
                current.push(kind, "\"");
            }
            TokenKind::StringPart => {
                current.push(kind, text);
            }
            TokenKind::InterpolationStart => {
                interp_depth += 1;
                current.push(kind, "#{");
            }
            TokenKind::StringEnd => {
                current.push(kind, "\"");
            }
            TokenKind::String => {
                current.push(kind, text);
            }

            // Paren tracking
            TokenKind::LParen => {
                paren_depth += 1;
                current.push(kind, "(");
            }
            TokenKind::RParen => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(kind, ")");
            }

            // Bracket tracking
            TokenKind::LBracket => {
                bracket_depth += 1;
                current.push(kind, "[");
            }
            TokenKind::RBracket => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(kind, "]");
            }

            // Brace tracking (struct/map literals)
            TokenKind::LBrace => {
                brace_depth += 1;
                current.push(kind, "{");
            }
            TokenKind::RBrace => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(kind, "}");
            }

            // `do`: always attaches to the preceding line, then breaks
            TokenKind::Do => {
                current.push(kind, "do");
                flush!();
            }

            // `end`: always on its own line
            TokenKind::End => {
                flush!();
                current.push(kind, "end");
                flush!();
            }

            // Block re-openers: else/rescue/catch/after
            // Always start a new line; may have trailing content before next break
            TokenKind::Else | TokenKind::Rescue | TokenKind::Catch | TokenKind::After => {
                flush!();
                current.push(kind, text);
            }

            // Arrow `->`: completes a branch head.
            // If the body is on the same source line, keep it inline.
            // If the body is on the next line, break after the arrow.
            TokenKind::Arrow => {
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

            // Pipe `|>`: break before pipe when not inline
            TokenKind::PipeGt if !inline => {
                flush!();
                current.push(kind, "|>");
            }

            // Semicolon: always breaks line
            TokenKind::Semicolon => {
                flush!();
            }

            // All other tokens: accumulate, breaking on source-line boundaries
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
                current.push(kind, text);
            }
        }

        prev_line_num = Some(tok_line);
        i += 1;
    }

    flush!();
    lines
}

/// Build a map: byte_offset → source line number (0-indexed).
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

/// Build a map: line_number → bool (true if there's a blank source line
/// before this line number, i.e., there was an empty line between this
/// line and the previous non-empty line).
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

/// Apply indentation to the logical lines, emitting each line with proper indent.
///
/// Uses a stateful algorithm that tracks whether we are in a "branch body"
/// (after a `->` arm head). When in a branch body, the next branch head or
/// block closer triggers a dedent.
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

        // PRE-EMIT indent adjustments
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

        // POST-EMIT indent adjustments
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

pub(super) fn format_tokens(source: &str, tokens: &[Token]) -> String {
    let lines = build_logical_lines(source, tokens);
    let mut output = apply_indentation(&lines);

    // Ensure no trailing blank lines, exactly one trailing newline
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
