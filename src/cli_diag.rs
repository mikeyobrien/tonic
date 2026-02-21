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
}
