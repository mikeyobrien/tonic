use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverDiagnosticCode {
    UndefinedSymbol,
}

impl ResolverDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UndefinedSymbol => "E1001",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolverError {
    code: ResolverDiagnosticCode,
    message: String,
}

impl ResolverError {
    pub fn undefined_symbol(symbol: &str, module: &str, function: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::UndefinedSymbol,
            message: format!("undefined symbol '{symbol}' in {module}.{function}"),
        }
    }

    #[cfg(test)]
    pub fn code(&self) -> ResolverDiagnosticCode {
        self.code
    }

    #[cfg(test)]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ResolverError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.code.as_str(), self.message)
    }
}

impl std::error::Error for ResolverError {}

#[cfg(test)]
mod tests {
    use super::{ResolverDiagnosticCode, ResolverError};

    #[test]
    fn undefined_symbol_constructor_uses_stable_code_and_message() {
        let error = ResolverError::undefined_symbol("missing", "Demo", "run");

        assert_eq!(error.code(), ResolverDiagnosticCode::UndefinedSymbol);
        assert_eq!(error.message(), "undefined symbol 'missing' in Demo.run");
        assert_eq!(
            error.to_string(),
            "[E1001] undefined symbol 'missing' in Demo.run"
        );
    }
}
