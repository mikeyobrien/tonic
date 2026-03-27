use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn expect_exact_args(
    function: &str,
    args: &[RuntimeValue],
    expected: usize,
) -> Result<(), HostError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "{} expects exactly {} argument{}, found {}",
            function,
            expected,
            if expected == 1 { "" } else { "s" },
            args.len()
        )))
    }
}

/// POSIX single-quote a string for safe shell use.
///
/// Wraps the value in single quotes, escaping any embedded single quotes
/// by ending the quote, inserting an escaped single quote, and reopening.
/// e.g. `it's` becomes `'it'"'"'s'`
fn shell_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\"'\"'");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

fn host_shell_quote(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Shell.quote", args, 1)?;
    let s = match &args[0] {
        RuntimeValue::String(s) => s,
        other => {
            return Err(HostError::new(format!(
                "Shell.quote expects a string argument, found {}",
                host_value_kind(other)
            )));
        }
    };
    Ok(RuntimeValue::String(shell_quote(s)))
}

fn host_shell_join(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Shell.join", args, 1)?;
    let items = match &args[0] {
        RuntimeValue::List(items) => items,
        other => {
            return Err(HostError::new(format!(
                "Shell.join expects a list argument, found {}",
                host_value_kind(other)
            )));
        }
    };
    let mut parts = Vec::with_capacity(items.len());
    for (i, item) in items.iter().enumerate() {
        match item {
            RuntimeValue::String(s) => parts.push(shell_quote(s)),
            other => {
                return Err(HostError::new(format!(
                    "Shell.join: element {} must be a string, found {}",
                    i,
                    host_value_kind(other)
                )));
            }
        }
    }
    Ok(RuntimeValue::String(parts.join(" ")))
}

pub fn register_shell_host_functions(registry: &HostRegistry) {
    registry.register("shell_quote", host_shell_quote);
    registry.register("shell_join", host_shell_join);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn shell_quote_simple_string() {
        let result = HOST_REGISTRY
            .call("shell_quote", &[s("hello")])
            .expect("shell_quote should succeed");
        assert_eq!(result, s("'hello'"));
    }

    #[test]
    fn shell_quote_empty_string() {
        let result = HOST_REGISTRY
            .call("shell_quote", &[s("")])
            .expect("shell_quote should succeed");
        assert_eq!(result, s("''"));
    }

    #[test]
    fn shell_quote_with_single_quotes() {
        let result = HOST_REGISTRY
            .call("shell_quote", &[s("it's")])
            .expect("shell_quote should succeed");
        assert_eq!(result, s("'it'\"'\"'s'"));
    }

    #[test]
    fn shell_quote_with_spaces_and_special_chars() {
        let result = HOST_REGISTRY
            .call("shell_quote", &[s("hello world $HOME")])
            .expect("shell_quote should succeed");
        assert_eq!(result, s("'hello world $HOME'"));
    }

    #[test]
    fn shell_quote_with_newlines() {
        let result = HOST_REGISTRY
            .call("shell_quote", &[s("line1\nline2")])
            .expect("shell_quote should succeed");
        assert_eq!(result, s("'line1\nline2'"));
    }

    #[test]
    fn shell_quote_with_double_quotes() {
        let result = HOST_REGISTRY
            .call("shell_quote", &[s("say \"hi\"")])
            .expect("shell_quote should succeed");
        assert_eq!(result, s("'say \"hi\"'"));
    }

    #[test]
    fn shell_quote_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("shell_quote", &[RuntimeValue::Int(42)])
            .expect_err("shell_quote should fail on non-string");
        assert!(
            err.to_string().contains("string"),
            "error should mention string: {err}"
        );
    }

    #[test]
    fn shell_join_simple_list() {
        let list = RuntimeValue::List(vec![s("echo"), s("hello"), s("world")]);
        let result = HOST_REGISTRY
            .call("shell_join", &[list])
            .expect("shell_join should succeed");
        assert_eq!(result, s("'echo' 'hello' 'world'"));
    }

    #[test]
    fn shell_join_empty_list() {
        let list = RuntimeValue::List(vec![]);
        let result = HOST_REGISTRY
            .call("shell_join", &[list])
            .expect("shell_join should succeed");
        assert_eq!(result, s(""));
    }

    #[test]
    fn shell_join_with_special_chars() {
        let list = RuntimeValue::List(vec![s("grep"), s("-r"), s("it's a $var")]);
        let result = HOST_REGISTRY
            .call("shell_join", &[list])
            .expect("shell_join should succeed");
        assert_eq!(result, s("'grep' '-r' 'it'\"'\"'s a $var'"));
    }

    #[test]
    fn shell_join_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("shell_join", &[s("not a list")])
            .expect_err("shell_join should fail on non-list");
        assert!(
            err.to_string().contains("list"),
            "error should mention list: {err}"
        );
    }

    #[test]
    fn shell_join_non_string_element_returns_error() {
        let list = RuntimeValue::List(vec![s("echo"), RuntimeValue::Int(42)]);
        let err = HOST_REGISTRY
            .call("shell_join", &[list])
            .expect_err("shell_join should fail on non-string element");
        assert!(
            err.to_string().contains("element 1"),
            "error should mention element index: {err}"
        );
    }
}
