pub const EXIT_OK: i32 = 0;
pub const EXIT_FAILURE: i32 = 1;
pub const EXIT_USAGE: i32 = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliDiagnostic {
    exit_code: i32,
    lines: Vec<String>,
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
        let mut diagnostic = Self::failure(message);

        if let Some(offset) = offset {
            if let Some(lines) = source_context_lines(source, offset) {
                diagnostic.lines.extend(lines);
            }
        }

        diagnostic
    }

    pub fn emit(self) -> i32 {
        for line in self.lines {
            eprintln!("{line}");
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

fn source_context_lines(source: &str, offset: usize) -> Option<Vec<String>> {
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

    Some(vec![
        format!(" --> line {line_number}, column {column_number}"),
        format!("{:>4} | {line_text}", line_number),
        format!("     | {caret_padding}^"),
    ])
}

#[cfg(test)]
mod tests {
    use super::{CliDiagnostic, EXIT_FAILURE, EXIT_USAGE};

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
}
