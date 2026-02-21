use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingDiagnosticCode {
    TypeMismatch,
    QuestionRequiresResult,
    NonExhaustiveCase,
}

impl TypingDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeMismatch => "E2001",
            Self::QuestionRequiresResult => "E3001",
            Self::NonExhaustiveCase => "E3002",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypingError {
    code: Option<TypingDiagnosticCode>,
    message: String,
    offset: Option<usize>,
}

impl TypingError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            code: None,
            message: message.into(),
            offset: None,
        }
    }

    pub fn type_mismatch(expected: &str, found: &str, offset: Option<usize>) -> Self {
        Self {
            code: Some(TypingDiagnosticCode::TypeMismatch),
            message: format!("type mismatch: expected {expected}, found {found}"),
            offset,
        }
    }

    pub fn question_requires_result(found: &str, offset: Option<usize>) -> Self {
        Self::result_match(
            TypingDiagnosticCode::QuestionRequiresResult,
            format!("? operator requires Result value, found {found}"),
            offset,
        )
    }

    pub fn non_exhaustive_case(offset: Option<usize>) -> Self {
        Self::result_match(
            TypingDiagnosticCode::NonExhaustiveCase,
            "non-exhaustive case expression: missing wildcard branch",
            offset,
        )
    }

    fn result_match(
        code: TypingDiagnosticCode,
        message: impl Into<String>,
        offset: Option<usize>,
    ) -> Self {
        Self {
            code: Some(code),
            message: message.into(),
            offset,
        }
    }

    #[cfg(test)]
    pub fn code(&self) -> Option<TypingDiagnosticCode> {
        self.code
    }

    #[cfg(test)]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for TypingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.code, self.offset) {
            (Some(code), Some(offset)) => {
                write!(f, "[{}] {} at offset {offset}", code.as_str(), self.message)
            }
            (Some(code), None) => write!(f, "[{}] {}", code.as_str(), self.message),
            (None, _) => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for TypingError {}

#[cfg(test)]
mod tests {
    use super::{TypingDiagnosticCode, TypingError};

    #[test]
    fn question_requires_result_constructor_uses_stable_contract() {
        let error = TypingError::question_requires_result("int", Some(74));

        assert_eq!(
            error.code(),
            Some(TypingDiagnosticCode::QuestionRequiresResult)
        );
        assert_eq!(
            error.message(),
            "? operator requires Result value, found int"
        );
        assert_eq!(
            error.to_string(),
            "[E3001] ? operator requires Result value, found int at offset 74"
        );
    }

    #[test]
    fn non_exhaustive_case_constructor_uses_stable_contract() {
        let error = TypingError::non_exhaustive_case(Some(37));

        assert_eq!(error.code(), Some(TypingDiagnosticCode::NonExhaustiveCase));
        assert_eq!(
            error.message(),
            "non-exhaustive case expression: missing wildcard branch"
        );
        assert_eq!(
            error.to_string(),
            "[E3002] non-exhaustive case expression: missing wildcard branch at offset 37"
        );
    }
}
