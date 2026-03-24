use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypingDiagnosticCode {
    TypeMismatch,
    ArityMismatch,
    QuestionRequiresResult,
    NonExhaustiveCase,
}

impl TypingDiagnosticCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TypeMismatch => "E2001",
            Self::ArityMismatch => "E2002",
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
        Self::type_mismatch_with_hint(expected, found, None, offset)
    }

    pub fn bool_type_mismatch(found: &str, offset: Option<usize>) -> Self {
        Self::type_mismatch_with_hint(
            "bool",
            found,
            Some("use a boolean expression here, for example `value != 0` or `is_nil(value)`"),
            offset,
        )
    }

    pub fn host_call_key_type_mismatch(found: &str, offset: Option<usize>) -> Self {
        Self::type_mismatch_with_hint(
            "atom",
            found,
            Some("pass an atom key as the first argument, for example `:sum_ints`"),
            offset,
        )
    }

    pub fn int_binary_operator_type_mismatch(
        operator: &str,
        side: &str,
        found: &str,
        offset: Option<usize>,
    ) -> Self {
        Self {
            code: Some(TypingDiagnosticCode::TypeMismatch),
            message: format!(
                "type mismatch: `{operator}` requires ints on both sides, found {found} on the {side}; hint: {}",
                Self::int_operator_hint(found, operator, true)
            ),
            offset,
        }
    }

    pub fn int_range_bound_type_mismatch(side: &str, found: &str, offset: Option<usize>) -> Self {
        Self {
            code: Some(TypingDiagnosticCode::TypeMismatch),
            message: format!(
                "type mismatch: `..` requires int bounds, found {found} on the {side}; hint: {}",
                Self::range_bound_hint(found)
            ),
            offset,
        }
    }

    pub fn int_unary_operator_type_mismatch(
        operator: &str,
        found: &str,
        offset: Option<usize>,
    ) -> Self {
        Self {
            code: Some(TypingDiagnosticCode::TypeMismatch),
            message: format!(
                "type mismatch: `{operator}` requires an int operand, found {found}; hint: {}",
                Self::int_operator_hint(found, operator, false)
            ),
            offset,
        }
    }

    pub fn arity_mismatch(
        target: &str,
        accepted_arities: &[usize],
        found: usize,
        offset: Option<usize>,
    ) -> Self {
        let expected = match accepted_arities {
            [] => "expected a supported number of args".to_string(),
            [arity] => format!(
                "expected {} {}, found {found}",
                arity,
                Self::arg_word(*arity)
            ),
            arities if Self::is_contiguous(arities) => {
                let min = arities[0];
                let max = arities[arities.len() - 1];
                format!("expected {min}..{max} args, found {found}")
            }
            arities => {
                let expected = arities
                    .iter()
                    .map(|arity| arity.to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("expected one of {expected} args, found {found}")
            }
        };

        let hint = match accepted_arities {
            [] => format!("adjust this call to match one of {target}'s supported arities"),
            [arity] => format!("call `{target}/{arity}`"),
            arities => format!(
                "use one of the accepted arities: {}",
                Self::format_accepted_arities(target, arities)
            ),
        };

        Self::arity_mismatch_with_hint(target, expected, hint, offset)
    }

    pub fn minimum_arity_mismatch(
        target: &str,
        minimum: usize,
        found: usize,
        hint: &str,
        offset: Option<usize>,
    ) -> Self {
        Self::arity_mismatch_with_hint(
            target,
            format!(
                "expected at least {minimum} {}, found {found}",
                Self::arg_word(minimum)
            ),
            hint,
            offset,
        )
    }

    pub fn question_requires_result(found: &str, hint: &str, offset: Option<usize>) -> Self {
        Self::result_match_with_hint(
            TypingDiagnosticCode::QuestionRequiresResult,
            format!("? operator requires Result value, found {found}"),
            hint,
            offset,
        )
    }

    pub fn non_exhaustive_case(offset: Option<usize>) -> Self {
        Self::result_match_with_hint(
            TypingDiagnosticCode::NonExhaustiveCase,
            "non-exhaustive case expression: missing wildcard branch",
            "add a catch-all branch such as `_ -> ...` to handle any remaining values",
            offset,
        )
    }

    fn type_mismatch_with_hint(
        expected: &str,
        found: &str,
        hint: Option<&str>,
        offset: Option<usize>,
    ) -> Self {
        let hint = hint
            .map(|hint| format!("; hint: {hint}"))
            .unwrap_or_default();
        Self {
            code: Some(TypingDiagnosticCode::TypeMismatch),
            message: format!("type mismatch: expected {expected}, found {found}{hint}"),
            offset,
        }
    }

    fn arity_mismatch_with_hint(
        target: &str,
        detail: impl Into<String>,
        hint: impl Into<String>,
        offset: Option<usize>,
    ) -> Self {
        Self {
            code: Some(TypingDiagnosticCode::ArityMismatch),
            message: format!(
                "arity mismatch for {target}: {}; hint: {}",
                detail.into(),
                hint.into()
            ),
            offset,
        }
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

    fn result_match_with_hint(
        code: TypingDiagnosticCode,
        message: impl Into<String>,
        hint: &str,
        offset: Option<usize>,
    ) -> Self {
        Self::result_match(code, format!("{}; hint: {hint}", message.into()), offset)
    }

    fn arg_word(count: usize) -> &'static str {
        if count == 1 {
            "arg"
        } else {
            "args"
        }
    }

    fn is_contiguous(arities: &[usize]) -> bool {
        arities
            .windows(2)
            .all(|window| window[0].saturating_add(1) == window[1])
    }

    fn format_accepted_arities(target: &str, arities: &[usize]) -> String {
        let forms = arities
            .iter()
            .map(|arity| format!("`{target}/{arity}`"))
            .collect::<Vec<_>>();

        match forms.as_slice() {
            [] => "".to_string(),
            [only] => only.clone(),
            [left, right] => format!("{left} or {right}"),
            _ => {
                let mut rendered = forms[..forms.len() - 1].join(", ");
                rendered.push_str(", or ");
                rendered.push_str(forms.last().expect("non-empty forms should have last"));
                rendered
            }
        }
    }

    fn int_operator_hint(found: &str, operator: &str, allow_boolean_logic_hint: bool) -> String {
        match found {
            "string" => {
                "convert the string to an int first, for example `String.to_integer(value)`"
                    .to_string()
            }
            "float" => {
                "round or truncate the float first, for example `round(value)` or `trunc(value)`"
                    .to_string()
            }
            "bool" if allow_boolean_logic_hint => {
                "replace the boolean operand with an int value, or use `and`/`or` for boolean logic"
                    .to_string()
            }
            "bool" => {
                format!("replace the boolean operand with an int before applying `{operator}`")
            }
            "nil" => format!("replace `nil` with an int before applying `{operator}`"),
            "result" => "unwrap the Result to an int before using this operator".to_string(),
            _ => format!("pass an int value before applying `{operator}`"),
        }
    }

    fn range_bound_hint(found: &str) -> String {
        match found {
            "string" => {
                "convert the string bound to an int first, for example `String.to_integer(value)`"
                    .to_string()
            }
            "float" => {
                "round or truncate the float bound first, for example `round(value)` or `trunc(value)`"
                    .to_string()
            }
            "bool" => "replace the boolean bound with an int such as `0` or `1`".to_string(),
            "nil" => "replace `nil` with an int bound before building the range".to_string(),
            "result" => "unwrap the Result to an int before building the range".to_string(),
            _ => "use an int value for each side of the range".to_string(),
        }
    }

    pub fn offset(&self) -> Option<usize> {
        self.offset
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
    fn arity_mismatch_constructor_uses_stable_contract() {
        let error = TypingError::arity_mismatch("Math.add", &[2], 1, Some(64));

        assert_eq!(error.code(), Some(TypingDiagnosticCode::ArityMismatch));
        assert_eq!(
            error.message(),
            "arity mismatch for Math.add: expected 2 args, found 1; hint: call `Math.add/2`"
        );
        assert_eq!(
            error.to_string(),
            "[E2002] arity mismatch for Math.add: expected 2 args, found 1; hint: call `Math.add/2` at offset 64"
        );
    }

    #[test]
    fn arity_range_mismatch_constructor_uses_stable_contract() {
        let error = TypingError::arity_mismatch("Demo.join", &[1, 2], 0, Some(29));

        assert_eq!(error.code(), Some(TypingDiagnosticCode::ArityMismatch));
        assert_eq!(
            error.message(),
            "arity mismatch for Demo.join: expected 1..2 args, found 0; hint: use one of the accepted arities: `Demo.join/1` or `Demo.join/2`"
        );
        assert_eq!(
            error.to_string(),
            "[E2002] arity mismatch for Demo.join: expected 1..2 args, found 0; hint: use one of the accepted arities: `Demo.join/1` or `Demo.join/2` at offset 29"
        );
    }

    #[test]
    fn question_requires_result_constructor_uses_stable_contract() {
        let error = TypingError::question_requires_result(
            "int",
            "wrap this value with `ok(...)` or `err(...)`, or remove the trailing `?`",
            Some(74),
        );

        assert_eq!(
            error.code(),
            Some(TypingDiagnosticCode::QuestionRequiresResult)
        );
        assert_eq!(
            error.message(),
            "? operator requires Result value, found int; hint: wrap this value with `ok(...)` or `err(...)`, or remove the trailing `?`"
        );
        assert_eq!(
            error.to_string(),
            "[E3001] ? operator requires Result value, found int; hint: wrap this value with `ok(...)` or `err(...)`, or remove the trailing `?` at offset 74"
        );
    }

    #[test]
    fn non_exhaustive_case_constructor_uses_stable_contract() {
        let error = TypingError::non_exhaustive_case(Some(37));

        assert_eq!(error.code(), Some(TypingDiagnosticCode::NonExhaustiveCase));
        assert_eq!(
            error.message(),
            "non-exhaustive case expression: missing wildcard branch; hint: add a catch-all branch such as `_ -> ...` to handle any remaining values"
        );
        assert_eq!(
            error.to_string(),
            "[E3002] non-exhaustive case expression: missing wildcard branch; hint: add a catch-all branch such as `_ -> ...` to handle any remaining values at offset 37"
        );
    }
}
