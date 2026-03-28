pub const EXIT_OK: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_USAGE: i32 = 64;

// ANSI color codes
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Whether to emit ANSI color codes when printing to stderr.
///
/// Respects the `NO_COLOR` environment variable convention.
/// Always disabled in test builds to keep assertion strings color-free.
#[cfg(not(test))]
fn colors_enabled() -> bool {
    use std::io::IsTerminal;
    std::env::var_os("NO_COLOR").is_none() && std::io::stderr().is_terminal()
}

#[cfg(test)]
fn colors_enabled() -> bool {
    false
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliDiagnostic {
    exit_code: i32,
    lines: Vec<String>,
}

pub(crate) fn failure_message_lines_with_filename_and_source(
    message: impl Into<String>,
    filename: Option<&str>,
    source: &str,
    offset: Option<usize>,
) -> Vec<String> {
    let message = message.into();
    let mut lines = vec![message];

    if let Some(offset) = offset {
        if let Some(context_lines) = source_context_lines(filename, source, offset) {
            lines.extend(context_lines);
        }
    }

    lines
}

impl CliDiagnostic {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_USAGE,
            lines: vec![format!("error: {}", message.into())],
        }
    }

    pub fn usage_with_hint(message: impl Into<String>, hint: impl Into<String>) -> Self {
        let mut diagnostic = Self::usage(message);
        diagnostic.lines.push(hint.into());
        diagnostic
    }

    pub fn failure(message: impl Into<String>) -> Self {
        Self {
            exit_code: EXIT_FAILURE,
            lines: vec![format!("error: {}", message.into())],
        }
    }

    pub fn failure_with_source(
        message: impl Into<String>,
        source: &str,
        offset: Option<usize>,
    ) -> Self {
        Self::failure_with_filename_and_source(message, None, source, offset)
    }

    /// Create a failure diagnostic with full source context.
    ///
    /// `filename` is shown in the `-->` location line when provided.
    pub fn failure_with_filename_and_source(
        message: impl Into<String>,
        filename: Option<&str>,
        source: &str,
        offset: Option<usize>,
    ) -> Self {
        let lines =
            failure_message_lines_with_filename_and_source(message, filename, source, offset);
        let mut diagnostic = Self::failure(lines[0].clone());
        diagnostic.lines.extend(lines.into_iter().skip(1));
        diagnostic
    }

    pub fn emit(self) -> i32 {
        for line in &self.lines {
            eprintln!("{}", colorize_line(line));
        }

        self.exit_code
    }

    #[cfg(test)]
    pub fn exit_code(&self) -> i32 {
        self.exit_code
    }

    #[cfg(test)]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

/// Apply ANSI colors to a single diagnostic output line.
///
/// - `error: ...` lines: bold red prefix
/// - ` --> ...` location lines: cyan
/// - `     | ...^` caret lines: red caret
/// - source snippet lines (`  N | ...`): plain
fn colorize_line(line: &str) -> String {
    if !colors_enabled() {
        return line.to_string();
    }

    if let Some(rest) = line.strip_prefix("error: ") {
        format!("{BOLD}{RED}error{RESET}{BOLD}:{RESET} {rest}")
    } else if line.starts_with(" --> ") {
        format!("{CYAN}{line}{RESET}")
    } else if line.ends_with('^') && line.contains('|') {
        let caret_pos = line.rfind('^').unwrap();
        let (prefix, caret) = line.split_at(caret_pos);
        format!("{prefix}{RED}{caret}{RESET}")
    } else {
        line.to_string()
    }
}

fn source_context_lines(
    filename: Option<&str>,
    source: &str,
    offset: usize,
) -> Option<Vec<String>> {
    if offset > source.len() || !source.is_char_boundary(offset) {
        return None;
    }

    let before = &source[..offset];
    let line_start = before.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let line_suffix = &source[offset..];
    let line_end = line_suffix
        .find('\n')
        .map(|relative_index| offset + relative_index)
        .unwrap_or(source.len());

    if !source.is_char_boundary(line_start) || !source.is_char_boundary(line_end) {
        return None;
    }

    let line_number = source[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let column_number = source[line_start..offset].chars().count() + 1;
    let line_text = &source[line_start..line_end];

    let mut caret_padding = String::new();
    for ch in source[line_start..offset].chars() {
        if ch == '\t' {
            caret_padding.push('\t');
        } else {
            caret_padding.push(' ');
        }
    }

    let location_line = match filename {
        Some(name) => format!(" --> {name}:{line_number}:{column_number}"),
        None => format!(" --> line {line_number}, column {column_number}"),
    };

    Some(vec![
        location_line,
        format!("{:>4} | {line_text}", line_number),
        format!("     | {caret_padding}^"),
    ])
}

#[cfg(test)]
mod tests {
    use super::{source_context_lines, CliDiagnostic, EXIT_FAILURE, EXIT_USAGE};

    #[test]
    fn usage_with_hint_sets_usage_exit_code_and_lines() {
        let diagnostic = CliDiagnostic::usage_with_hint("unknown command", "run tonic --help");

        assert_eq!(diagnostic.exit_code(), EXIT_USAGE);
        assert_eq!(
            diagnostic.lines(),
            [
                "error: unknown command".to_string(),
                "run tonic --help".to_string(),
            ]
        );
    }

    #[test]
    fn failure_sets_failure_exit_code() {
        let diagnostic = CliDiagnostic::failure("missing acceptance file acceptance/step-01.yaml");

        assert_eq!(diagnostic.exit_code(), EXIT_FAILURE);
        assert_eq!(
            diagnostic.lines(),
            ["error: missing acceptance file acceptance/step-01.yaml".to_string()]
        );
    }

    #[test]
    fn failure_with_source_appends_line_column_and_snippet() {
        let source = "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n";
        let diagnostic = CliDiagnostic::failure_with_source(
            "[E1001] undefined symbol 'missing'",
            source,
            Some(37),
        );

        assert_eq!(diagnostic.exit_code(), EXIT_FAILURE);
        assert_eq!(
            diagnostic.lines(),
            [
                "error: [E1001] undefined symbol 'missing'".to_string(),
                " --> line 3, column 5".to_string(),
                "   3 |     missing()".to_string(),
                "     |     ^".to_string(),
            ]
        );
    }

    #[test]
    fn failure_with_filename_and_source_renders_filename_in_location_line() {
        let source = "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n";
        let diagnostic = CliDiagnostic::failure_with_filename_and_source(
            "[E1001] undefined symbol 'missing'",
            Some("examples/demo.tn"),
            source,
            Some(37),
        );

        assert_eq!(diagnostic.exit_code(), EXIT_FAILURE);
        assert_eq!(
            diagnostic.lines(),
            [
                "error: [E1001] undefined symbol 'missing'".to_string(),
                " --> examples/demo.tn:3:5".to_string(),
                "   3 |     missing()".to_string(),
                "     |     ^".to_string(),
            ]
        );
    }

    #[test]
    fn source_context_lines_with_filename_uses_colon_format() {
        let source = "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n";
        let lines = source_context_lines(Some("src/demo.tn"), source, 37).unwrap();

        assert_eq!(lines[0], " --> src/demo.tn:3:5");
    }

    #[test]
    fn source_context_lines_without_filename_uses_prose_format() {
        let source = "defmodule Demo do\n  def run() do\n    missing()\n  end\nend\n";
        let lines = source_context_lines(None, source, 37).unwrap();

        assert_eq!(lines[0], " --> line 3, column 5");
    }

    #[test]
    fn source_context_lines_at_column_one() {
        let source = "x = 1\ny = missing\n";
        let lines = source_context_lines(Some("demo.tn"), source, 6).unwrap();

        assert_eq!(lines[0], " --> demo.tn:2:1");
        assert_eq!(lines[2], "     | ^");
    }

    #[test]
    fn failure_with_filename_and_source_no_offset_has_no_context() {
        let source = "defmodule Demo do\nend\n";
        let diagnostic = CliDiagnostic::failure_with_filename_and_source(
            "some error",
            Some("demo.tn"),
            source,
            None,
        );

        assert_eq!(diagnostic.lines(), ["error: some error".to_string()]);
    }
}
