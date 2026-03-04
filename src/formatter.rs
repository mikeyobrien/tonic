use crate::lexer::{scan_tokens, Token, TokenKind};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormatMode {
    Write,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FormatReport {
    pub(crate) checked_files: usize,
    pub(crate) changed_files: usize,
}

pub(crate) fn format_path(path: &str, mode: FormatMode) -> Result<FormatReport, String> {
    let requested_path = Path::new(path);

    if !requested_path.exists() {
        return Err(format!("path does not exist: {}", requested_path.display()));
    }

    let files = collect_tonic_files(requested_path)?;
    if files.is_empty() {
        return Err(format!(
            "no .tn source files found at {}",
            requested_path.display()
        ));
    }

    let mut report = FormatReport {
        checked_files: 0,
        changed_files: 0,
    };

    for file in files {
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read source file {}: {error}", file.display()))?;
        let formatted = format_source(&source);

        report.checked_files += 1;

        if source != formatted {
            report.changed_files += 1;

            if mode == FormatMode::Write {
                fs::write(&file, formatted).map_err(|error| {
                    format!("failed to write formatted file {}: {error}", file.display())
                })?;
            }
        }
    }

    Ok(report)
}

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
pub(crate) fn format_source(source: &str) -> String {
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
struct LogicalLine {
    /// Token kinds for classification
    kinds: Vec<TokenKind>,
    /// The rendered text of the line (without indentation)
    text: String,
    /// Whether this line has a blank line gap before it (from source)
    blank_before: bool,
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
        // Determine spacing
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

    // Pipe operator lines are emitted as their own segment, so PipeGt shouldn't
    // appear mid-line normally; if it does, space both sides.
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
fn build_logical_lines(source: &str, tokens: &[Token]) -> Vec<LogicalLine> {
    // Pre-compute which source line each byte offset is on.
    // line_for_offset[i] = line number (0-indexed) of byte i.
    let line_for_offset = compute_line_map(source);

    let mut lines: Vec<LogicalLine> = Vec::new();
    let mut current = LogicalLine::new();
    let mut interp_depth: usize = 0;
    let mut paren_depth: usize = 0;
    let mut bracket_depth: usize = 0;
    let mut brace_depth: usize = 0;
    let mut prev_line_num: Option<usize> = None;

    // Track blank lines: was there a blank source line before the token's line?
    // blank_before_line[n] = true if source line n is preceded by at least one blank line
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

        // Determine if we're inside any inline grouping
        let inline = paren_depth > 0 || bracket_depth > 0 || brace_depth > 0;

        match kind {
            // -------------------------------------------------------
            // String literals: accumulate inline
            // -------------------------------------------------------
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

            // -------------------------------------------------------
            // Paren tracking
            // -------------------------------------------------------
            TokenKind::LParen => {
                paren_depth += 1;
                current.push(kind, "(");
            }
            TokenKind::RParen => {
                paren_depth = paren_depth.saturating_sub(1);
                current.push(kind, ")");
            }

            // -------------------------------------------------------
            // Bracket tracking
            // -------------------------------------------------------
            TokenKind::LBracket => {
                bracket_depth += 1;
                current.push(kind, "[");
            }
            TokenKind::RBracket => {
                bracket_depth = bracket_depth.saturating_sub(1);
                current.push(kind, "]");
            }

            // -------------------------------------------------------
            // Brace tracking (struct/map literals)
            // -------------------------------------------------------
            TokenKind::LBrace => {
                brace_depth += 1;
                current.push(kind, "{");
            }
            TokenKind::RBrace => {
                brace_depth = brace_depth.saturating_sub(1);
                current.push(kind, "}");
            }

            // -------------------------------------------------------
            // `do`: always attaches to the preceding line, then breaks
            // -------------------------------------------------------
            TokenKind::Do => {
                current.push(kind, "do");
                flush!();
            }

            // -------------------------------------------------------
            // `end`: always on its own line
            // -------------------------------------------------------
            TokenKind::End => {
                flush!();
                current.push(kind, "end");
                flush!();
            }

            // -------------------------------------------------------
            // Block re-openers: else/rescue/catch/after
            // Always start a new line; may have trailing content before next break
            // -------------------------------------------------------
            TokenKind::Else | TokenKind::Rescue | TokenKind::Catch | TokenKind::After => {
                flush!();
                current.push(kind, text);
                // Don't flush immediately - may have trailing pattern before ->
                // The next structural break (do, end, ->, source line) will flush.
            }

            // -------------------------------------------------------
            // Arrow `->`: completes a branch head.
            // If the body is on the same source line, keep it inline.
            // If the body is on the next line, break after the arrow.
            // -------------------------------------------------------
            TokenKind::Arrow => {
                current.push(kind, "->");
                // Look at the next token to decide if we break here
                let next_tok = tokens.get(i + 1);
                let break_after = match next_tok {
                    None => true,
                    Some(next) => {
                        let next_kind = next.kind();
                        // Always break if next is a structural token
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
                            // Break if next token is on a different source line
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

            // -------------------------------------------------------
            // Pipe `|>`: break before pipe when not inline
            // -------------------------------------------------------
            TokenKind::PipeGt if !inline => {
                flush!();
                current.push(kind, "|>");
                // The body of the pipe expression continues on the same logical line
            }

            // -------------------------------------------------------
            // Semicolon: always breaks line
            // -------------------------------------------------------
            TokenKind::Semicolon => {
                flush!();
            }

            // -------------------------------------------------------
            // All other tokens: accumulate
            // If not inline and token is on a different source line than
            // the previous token, break before it.
            // -------------------------------------------------------
            _ => {
                if !inline {
                    if let Some(prev_ln) = prev_line_num {
                        if tok_line > prev_ln {
                            flush!();
                            // Track blank lines: if any source line between prev_ln and
                            // tok_line was blank, mark the new line with blank_before
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
    map.push(line); // one extra for EOF
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
fn apply_indentation(lines: &[LogicalLine]) -> String {
    let mut output = String::new();
    let mut indent: i32 = 0;
    let mut in_branch_body = false; // true after emitting a branch head (ends with ->)

    for line in lines {
        // Blank line before this line?
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
        let is_branch_head = ends_arrow; // any line ending with -> is a branch head

        // PRE-EMIT indent adjustments:
        if is_block_reopen {
            // De-indent from current body (branch body or do body)
            if in_branch_body {
                indent = indent.saturating_sub(1);
                in_branch_body = false;
            }
            indent = indent.saturating_sub(1);
        } else if is_end {
            // De-indent from current body (branch body or do body)
            if in_branch_body {
                indent = indent.saturating_sub(1);
                in_branch_body = false;
            }
            indent = indent.saturating_sub(1);
        } else if is_branch_head && in_branch_body {
            // This branch head starts a new arm; de-indent from previous body
            indent = indent.saturating_sub(1);
            in_branch_body = false;
        }

        if indent < 0 {
            indent = 0;
        }

        // Emit the line
        let indent_str = "  ".repeat(indent as usize);
        output.push_str(&indent_str);
        output.push_str(&line.text);
        output.push('\n');

        // POST-EMIT indent adjustments:
        if is_block_reopen {
            // The block re-opener opens a new body section (e.g., else body)
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

fn format_tokens(source: &str, tokens: &[Token]) -> String {
    let lines = build_logical_lines(source, tokens);
    let mut output = apply_indentation(&lines);

    // Ensure no trailing blank lines
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

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

fn collect_tonic_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut files = Vec::new();
    collect_tonic_files_recursive(path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_tonic_files_recursive(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read directory {}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!(
                "failed to read directory entry in {}: {error}",
                directory.display()
            )
        })?;

    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();

        if path.is_dir() {
            collect_tonic_files_recursive(&path, files)?;
            continue;
        }

        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tn"))
        {
            files.push(path);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Utilities
// ---------------------------------------------------------------------------

fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn format_source_indents_nested_do_blocks() {
        let source = "defmodule Demo do\ndef run() do\nif true do\n1\nelse\n2\nend\nend\nend\n";

        assert_eq!(
            format_source(source),
            "defmodule Demo do\n  def run() do\n    if true do\n      1\n    else\n      2\n    end\n  end\nend\n"
        );
    }

    #[test]
    fn format_source_indents_case_branches() {
        let source =
            "defmodule Demo do\ndef run() do\ncase 2 do\n1 ->\n10\n2 ->\n20\nend\nend\nend\n";

        assert_eq!(
            format_source(source),
            "defmodule Demo do\n  def run() do\n    case 2 do\n      1 ->\n        10\n      2 ->\n        20\n    end\n  end\nend\n"
        );
    }

    #[test]
    fn format_source_collapses_extra_blank_lines() {
        let source = "defmodule Demo do\n\n\n  def run() do\n    1\n  end\nend\n\n";

        assert_eq!(
            format_source(source),
            "defmodule Demo do\n\n  def run() do\n    1\n  end\nend\n"
        );
    }

    #[test]
    fn format_source_is_idempotent_nested_if() {
        let already = "defmodule Demo do\n  def run() do\n    if true do\n      1\n    else\n      2\n    end\n  end\nend\n";
        let second = format_source(already);
        assert_eq!(
            already, second,
            "formatting already-formatted code must be idempotent"
        );
    }

    #[test]
    fn format_source_is_idempotent_case_branches() {
        let already = "defmodule Demo do\n  def run() do\n    case 2 do\n      1 ->\n        10\n      2 ->\n        20\n    end\n  end\nend\n";
        let second = format_source(already);
        assert_eq!(
            already, second,
            "formatting already-formatted code must be idempotent"
        );
    }

    #[test]
    fn format_source_struct_syntax_round_trip() {
        let source = "defmodule User do\ndefstruct name: \"\", age: 0\ndef run(user) do\ncase %User{user | age: 43} do\n%User{name: name} ->\n%User{name: name}\n_ ->\n%User{}\nend\nend\nend\n";
        let first = format_source(source);
        let second = format_source(&first);
        assert_eq!(first, second, "struct syntax format must be idempotent");
    }

    #[test]
    fn format_source_try_rescue_indented() {
        let source = "defmodule Demo do\ndef run() do\ntry do\nraise \"err\"\nrescue\n_ -> \"caught\"\nend\nend\nend\n";
        let result = format_source(source);
        let second = format_source(&result);
        assert_eq!(result, second, "try/rescue format must be idempotent");
    }

    #[test]
    fn format_source_function_clauses_idempotent() {
        let already = "defmodule Demo do\n  defp fib(0) do\n    0\n  end\n\n  defp fib(1) do\n    1\n  end\n\n  defp fib(n) when n > 1 do\n    fib(n - 1) + fib(n - 2)\n  end\nend\n";
        let second = format_source(already);
        assert_eq!(
            already, second,
            "function clauses with blank lines must be idempotent"
        );
    }
}
