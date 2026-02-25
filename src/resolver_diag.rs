use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverDiagnosticCode {
    UndefinedSymbol,
    PrivateFunction,
    DuplicateModule,
    UndefinedStructModule,
    UnknownStructField,
}

impl ResolverDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UndefinedSymbol => "E1001",
            Self::PrivateFunction => "E1002",
            Self::DuplicateModule => "E1003",
            Self::UndefinedStructModule => "E1004",
            Self::UnknownStructField => "E1005",
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

    pub fn private_function(symbol: &str, module: &str, function: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::PrivateFunction,
            message: format!(
                "private function '{symbol}' cannot be called from {module}.{function}"
            ),
        }
    }

    pub fn duplicate_module(module: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::DuplicateModule,
            message: format!("duplicate module definition '{module}'"),
        }
    }

    pub fn undefined_struct_module(struct_module: &str, module: &str, function: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::UndefinedStructModule,
            message: format!("undefined struct module '{struct_module}' in {module}.{function}"),
        }
    }

    pub fn unknown_struct_field(
        field: &str,
        struct_module: &str,
        module: &str,
        function: &str,
    ) -> Self {
        Self {
            code: ResolverDiagnosticCode::UnknownStructField,
            message: format!(
                "unknown struct field '{field}' for {struct_module} in {module}.{function}"
            ),
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

    #[test]
    fn private_function_constructor_uses_stable_code_and_message() {
        let error = ResolverError::private_function("Math.hidden", "Demo", "run");

        assert_eq!(error.code(), ResolverDiagnosticCode::PrivateFunction);
        assert_eq!(
            error.message(),
            "private function 'Math.hidden' cannot be called from Demo.run"
        );
        assert_eq!(
            error.to_string(),
            "[E1002] private function 'Math.hidden' cannot be called from Demo.run"
        );
    }

    #[test]
    fn duplicate_module_constructor_uses_stable_code_and_message() {
        let error = ResolverError::duplicate_module("Shared");

        assert_eq!(error.code(), ResolverDiagnosticCode::DuplicateModule);
        assert_eq!(error.message(), "duplicate module definition 'Shared'");
        assert_eq!(
            error.to_string(),
            "[E1003] duplicate module definition 'Shared'"
        );
    }

    #[test]
    fn undefined_struct_module_constructor_uses_stable_code_and_message() {
        let error = ResolverError::undefined_struct_module("User", "Demo", "run");

        assert_eq!(error.code(), ResolverDiagnosticCode::UndefinedStructModule);
        assert_eq!(
            error.message(),
            "undefined struct module 'User' in Demo.run"
        );
        assert_eq!(
            error.to_string(),
            "[E1004] undefined struct module 'User' in Demo.run"
        );
    }

    #[test]
    fn unknown_struct_field_constructor_uses_stable_code_and_message() {
        let error = ResolverError::unknown_struct_field("agez", "User", "Demo", "run");

        assert_eq!(error.code(), ResolverDiagnosticCode::UnknownStructField);
        assert_eq!(
            error.message(),
            "unknown struct field 'agez' for User in Demo.run"
        );
        assert_eq!(
            error.to_string(),
            "[E1005] unknown struct field 'agez' for User in Demo.run"
        );
    }
}
