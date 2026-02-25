use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolverDiagnosticCode {
    UndefinedSymbol,
    PrivateFunction,
    DuplicateModule,
    UndefinedStructModule,
    UnknownStructField,
    DuplicateProtocol,
    DuplicateProtocolFunction,
    UnknownProtocol,
    DuplicateProtocolImpl,
    InvalidProtocolImpl,
    UndefinedRequiredModule,
    UndefinedUseModule,
    ImportFilterExcludesCall,
    AmbiguousImportCall,
}

impl ResolverDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UndefinedSymbol => "E1001",
            Self::PrivateFunction => "E1002",
            Self::DuplicateModule => "E1003",
            Self::UndefinedStructModule => "E1004",
            Self::UnknownStructField => "E1005",
            Self::DuplicateProtocol => "E1006",
            Self::DuplicateProtocolFunction => "E1007",
            Self::UnknownProtocol => "E1008",
            Self::DuplicateProtocolImpl => "E1009",
            Self::InvalidProtocolImpl => "E1010",
            Self::UndefinedRequiredModule => "E1011",
            Self::UndefinedUseModule => "E1012",
            Self::ImportFilterExcludesCall => "E1013",
            Self::AmbiguousImportCall => "E1014",
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

    pub fn duplicate_protocol(protocol: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::DuplicateProtocol,
            message: format!("duplicate protocol definition '{protocol}'"),
        }
    }

    pub fn duplicate_protocol_function(protocol: &str, function: &str, arity: usize) -> Self {
        Self {
            code: ResolverDiagnosticCode::DuplicateProtocolFunction,
            message: format!(
                "duplicate protocol function '{function}/{arity}' in protocol '{protocol}'"
            ),
        }
    }

    pub fn unknown_protocol(protocol: &str, target: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::UnknownProtocol,
            message: format!("unknown protocol '{protocol}' for defimpl target '{target}'"),
        }
    }

    pub fn duplicate_protocol_impl(protocol: &str, target: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::DuplicateProtocolImpl,
            message: format!("duplicate defimpl for protocol '{protocol}' and target '{target}'"),
        }
    }

    pub fn invalid_protocol_impl(
        protocol: &str,
        target: &str,
        function: &str,
        arity: usize,
        reason: &str,
    ) -> Self {
        Self {
            code: ResolverDiagnosticCode::InvalidProtocolImpl,
            message: format!(
                "invalid defimpl for protocol '{protocol}' target '{target}': {function}/{arity} {reason}"
            ),
        }
    }

    pub fn undefined_required_module(required_module: &str, module: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::UndefinedRequiredModule,
            message: format!(
                "required module '{required_module}' is not defined for {module}; add the module or remove require"
            ),
        }
    }

    pub fn undefined_use_module(used_module: &str, module: &str) -> Self {
        Self {
            code: ResolverDiagnosticCode::UndefinedUseModule,
            message: format!(
                "used module '{used_module}' is not defined for {module}; add the module or remove use"
            ),
        }
    }

    pub fn import_filter_excludes_call(
        function: &str,
        arity: usize,
        module: &str,
        import_modules: &[String],
    ) -> Self {
        let imports = import_modules.join(", ");
        Self {
            code: ResolverDiagnosticCode::ImportFilterExcludesCall,
            message: format!(
                "import filters exclude call '{function}/{arity}' in {module}; imported modules with this symbol: {imports}"
            ),
        }
    }

    pub fn ambiguous_import_call(
        function: &str,
        arity: usize,
        module: &str,
        candidates: &[String],
    ) -> Self {
        let joined = candidates.join(", ");
        Self {
            code: ResolverDiagnosticCode::AmbiguousImportCall,
            message: format!(
                "ambiguous imported call '{function}/{arity}' in {module}; matches: {joined}"
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

    #[test]
    fn duplicate_protocol_constructor_uses_stable_code_and_message() {
        let error = ResolverError::duplicate_protocol("Size");

        assert_eq!(error.code(), ResolverDiagnosticCode::DuplicateProtocol);
        assert_eq!(error.message(), "duplicate protocol definition 'Size'");
        assert_eq!(
            error.to_string(),
            "[E1006] duplicate protocol definition 'Size'"
        );
    }

    #[test]
    fn duplicate_protocol_function_constructor_uses_stable_code_and_message() {
        let error = ResolverError::duplicate_protocol_function("Size", "size", 1);

        assert_eq!(
            error.code(),
            ResolverDiagnosticCode::DuplicateProtocolFunction
        );
        assert_eq!(
            error.message(),
            "duplicate protocol function 'size/1' in protocol 'Size'"
        );
        assert_eq!(
            error.to_string(),
            "[E1007] duplicate protocol function 'size/1' in protocol 'Size'"
        );
    }

    #[test]
    fn unknown_protocol_constructor_uses_stable_code_and_message() {
        let error = ResolverError::unknown_protocol("Size", "Tuple");

        assert_eq!(error.code(), ResolverDiagnosticCode::UnknownProtocol);
        assert_eq!(
            error.message(),
            "unknown protocol 'Size' for defimpl target 'Tuple'"
        );
        assert_eq!(
            error.to_string(),
            "[E1008] unknown protocol 'Size' for defimpl target 'Tuple'"
        );
    }

    #[test]
    fn duplicate_protocol_impl_constructor_uses_stable_code_and_message() {
        let error = ResolverError::duplicate_protocol_impl("Size", "Tuple");

        assert_eq!(error.code(), ResolverDiagnosticCode::DuplicateProtocolImpl);
        assert_eq!(
            error.message(),
            "duplicate defimpl for protocol 'Size' and target 'Tuple'"
        );
        assert_eq!(
            error.to_string(),
            "[E1009] duplicate defimpl for protocol 'Size' and target 'Tuple'"
        );
    }

    #[test]
    fn invalid_protocol_impl_constructor_uses_stable_code_and_message() {
        let error =
            ResolverError::invalid_protocol_impl("Size", "Tuple", "size", 1, "is not declared");

        assert_eq!(error.code(), ResolverDiagnosticCode::InvalidProtocolImpl);
        assert_eq!(
            error.message(),
            "invalid defimpl for protocol 'Size' target 'Tuple': size/1 is not declared"
        );
        assert_eq!(
            error.to_string(),
            "[E1010] invalid defimpl for protocol 'Size' target 'Tuple': size/1 is not declared"
        );
    }

    #[test]
    fn undefined_required_module_constructor_uses_stable_code_and_message() {
        let error = ResolverError::undefined_required_module("Logger", "Demo");

        assert_eq!(
            error.code(),
            ResolverDiagnosticCode::UndefinedRequiredModule
        );
        assert_eq!(
            error.message(),
            "required module 'Logger' is not defined for Demo; add the module or remove require"
        );
        assert_eq!(
            error.to_string(),
            "[E1011] required module 'Logger' is not defined for Demo; add the module or remove require"
        );
    }

    #[test]
    fn undefined_use_module_constructor_uses_stable_code_and_message() {
        let error = ResolverError::undefined_use_module("Feature", "Demo");

        assert_eq!(error.code(), ResolverDiagnosticCode::UndefinedUseModule);
        assert_eq!(
            error.message(),
            "used module 'Feature' is not defined for Demo; add the module or remove use"
        );
        assert_eq!(
            error.to_string(),
            "[E1012] used module 'Feature' is not defined for Demo; add the module or remove use"
        );
    }

    #[test]
    fn import_filter_excludes_call_constructor_uses_stable_code_and_message() {
        let error = ResolverError::import_filter_excludes_call(
            "helper",
            1,
            "Demo",
            &["Math".to_string(), "Helpers".to_string()],
        );

        assert_eq!(
            error.code(),
            ResolverDiagnosticCode::ImportFilterExcludesCall
        );
        assert_eq!(
            error.message(),
            "import filters exclude call 'helper/1' in Demo; imported modules with this symbol: Math, Helpers"
        );
        assert_eq!(
            error.to_string(),
            "[E1013] import filters exclude call 'helper/1' in Demo; imported modules with this symbol: Math, Helpers"
        );
    }

    #[test]
    fn ambiguous_import_call_constructor_uses_stable_code_and_message() {
        let error = ResolverError::ambiguous_import_call(
            "helper",
            1,
            "Demo",
            &["Math".to_string(), "Helpers".to_string()],
        );

        assert_eq!(error.code(), ResolverDiagnosticCode::AmbiguousImportCall);
        assert_eq!(
            error.message(),
            "ambiguous imported call 'helper/1' in Demo; matches: Math, Helpers"
        );
        assert_eq!(
            error.to_string(),
            "[E1014] ambiguous imported call 'helper/1' in Demo; matches: Math, Helpers"
        );
    }
}
