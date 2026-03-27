use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use regex::Regex;

fn expect_args(function: &str, args: &[RuntimeValue], expected: usize) -> Result<(), HostError> {
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

fn extract_string<'a>(
    function: &str,
    args: &'a [RuntimeValue],
    idx: usize,
) -> Result<&'a str, HostError> {
    match &args[idx] {
        RuntimeValue::String(s) => Ok(s.as_str()),
        other => Err(HostError::new(format!(
            "{} expects a string argument at position {}, found {}",
            function,
            idx + 1,
            host_value_kind(other)
        ))),
    }
}

fn compile_regex(function: &str, pattern: &str) -> Result<Regex, HostError> {
    Regex::new(pattern)
        .map_err(|e| HostError::new(format!("{}: invalid regex pattern: {}", function, e)))
}

/// Regex.match?/2 — test if string matches a pattern
fn host_regex_match(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Regex.match?", args, 2)?;
    let string = extract_string("Regex.match?", args, 0)?;
    let pattern = extract_string("Regex.match?", args, 1)?;
    let re = compile_regex("Regex.match?", pattern)?;
    Ok(RuntimeValue::Bool(re.is_match(string)))
}

/// Regex.run/2 — find first match, return list of captures (or nil)
fn host_regex_run(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Regex.run", args, 2)?;
    let string = extract_string("Regex.run", args, 0)?;
    let pattern = extract_string("Regex.run", args, 1)?;
    let re = compile_regex("Regex.run", pattern)?;

    match re.captures(string) {
        None => Ok(RuntimeValue::Nil),
        Some(caps) => {
            let captures: Vec<RuntimeValue> = caps
                .iter()
                .map(|m| match m {
                    Some(m) => RuntimeValue::String(m.as_str().to_string()),
                    None => RuntimeValue::Nil,
                })
                .collect();
            Ok(RuntimeValue::List(captures))
        }
    }
}

/// Regex.scan/2 — find all matches, return list of capture lists
fn host_regex_scan(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Regex.scan", args, 2)?;
    let string = extract_string("Regex.scan", args, 0)?;
    let pattern = extract_string("Regex.scan", args, 1)?;
    let re = compile_regex("Regex.scan", pattern)?;

    let matches: Vec<RuntimeValue> = re
        .captures_iter(string)
        .map(|caps| {
            // Skip group 0 (full match), return only capture groups
            // If no capture groups, return the full match
            let groups: Vec<RuntimeValue> = if re.captures_len() > 1 {
                caps.iter()
                    .skip(1)
                    .map(|m| match m {
                        Some(m) => RuntimeValue::String(m.as_str().to_string()),
                        None => RuntimeValue::Nil,
                    })
                    .collect()
            } else {
                vec![RuntimeValue::String(
                    caps.get(0).map_or("", |m| m.as_str()).to_string(),
                )]
            };
            RuntimeValue::List(groups)
        })
        .collect();
    Ok(RuntimeValue::List(matches))
}

/// Regex.replace/3 — replace first match
fn host_regex_replace(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Regex.replace", args, 3)?;
    let string = extract_string("Regex.replace", args, 0)?;
    let pattern = extract_string("Regex.replace", args, 1)?;
    let replacement = extract_string("Regex.replace", args, 2)?;
    let re = compile_regex("Regex.replace", pattern)?;
    Ok(RuntimeValue::String(
        re.replace(string, replacement).into_owned(),
    ))
}

/// Regex.replace_all/3 — replace all matches
fn host_regex_replace_all(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Regex.replace_all", args, 3)?;
    let string = extract_string("Regex.replace_all", args, 0)?;
    let pattern = extract_string("Regex.replace_all", args, 1)?;
    let replacement = extract_string("Regex.replace_all", args, 2)?;
    let re = compile_regex("Regex.replace_all", pattern)?;
    Ok(RuntimeValue::String(
        re.replace_all(string, replacement).into_owned(),
    ))
}

/// Regex.split/2 — split string by regex
fn host_regex_split(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_args("Regex.split", args, 2)?;
    let string = extract_string("Regex.split", args, 0)?;
    let pattern = extract_string("Regex.split", args, 1)?;
    let re = compile_regex("Regex.split", pattern)?;
    let parts: Vec<RuntimeValue> = re
        .split(string)
        .map(|s| RuntimeValue::String(s.to_string()))
        .collect();
    Ok(RuntimeValue::List(parts))
}

pub fn register_regex_host_functions(registry: &HostRegistry) {
    registry.register("regex_match", host_regex_match);
    registry.register("regex_run", host_regex_run);
    registry.register("regex_scan", host_regex_scan);
    registry.register("regex_replace", host_regex_replace);
    registry.register("regex_replace_all", host_regex_replace_all);
    registry.register("regex_split", host_regex_split);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn match_true() {
        let result = HOST_REGISTRY
            .call("regex_match", &[s("hello world"), s(r"\w+")])
            .expect("match should succeed");
        assert_eq!(result, RuntimeValue::Bool(true));
    }

    #[test]
    fn match_false() {
        let result = HOST_REGISTRY
            .call("regex_match", &[s("hello"), s(r"^\d+$")])
            .expect("match should succeed");
        assert_eq!(result, RuntimeValue::Bool(false));
    }

    #[test]
    fn run_with_captures() {
        let result = HOST_REGISTRY
            .call(
                "regex_run",
                &[s("2026-03-27"), s(r"(\d{4})-(\d{2})-(\d{2})")],
            )
            .expect("run should succeed");
        match result {
            RuntimeValue::List(caps) => {
                assert_eq!(caps.len(), 4); // full match + 3 groups
                assert_eq!(caps[0], s("2026-03-27"));
                assert_eq!(caps[1], s("2026"));
                assert_eq!(caps[2], s("03"));
                assert_eq!(caps[3], s("27"));
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn run_no_match_returns_nil() {
        let result = HOST_REGISTRY
            .call("regex_run", &[s("hello"), s(r"^\d+$")])
            .expect("run should succeed");
        assert_eq!(result, RuntimeValue::Nil);
    }

    #[test]
    fn scan_multiple_matches() {
        let result = HOST_REGISTRY
            .call("regex_scan", &[s("cat bat hat"), s(r"[cbh]at")])
            .expect("scan should succeed");
        match result {
            RuntimeValue::List(matches) => {
                assert_eq!(matches.len(), 3);
                // No capture groups → each match is [full_match]
                assert_eq!(matches[0], RuntimeValue::List(vec![s("cat")]));
                assert_eq!(matches[1], RuntimeValue::List(vec![s("bat")]));
                assert_eq!(matches[2], RuntimeValue::List(vec![s("hat")]));
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn scan_with_capture_groups() {
        let result = HOST_REGISTRY
            .call("regex_scan", &[s("a1 b2 c3"), s(r"([a-z])(\d)")])
            .expect("scan should succeed");
        match result {
            RuntimeValue::List(matches) => {
                assert_eq!(matches.len(), 3);
                assert_eq!(matches[0], RuntimeValue::List(vec![s("a"), s("1")]));
                assert_eq!(matches[1], RuntimeValue::List(vec![s("b"), s("2")]));
                assert_eq!(matches[2], RuntimeValue::List(vec![s("c"), s("3")]));
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn replace_first() {
        let result = HOST_REGISTRY
            .call("regex_replace", &[s("foo bar foo"), s("foo"), s("baz")])
            .expect("replace should succeed");
        assert_eq!(result, s("baz bar foo"));
    }

    #[test]
    fn replace_all() {
        let result = HOST_REGISTRY
            .call("regex_replace_all", &[s("foo bar foo"), s("foo"), s("baz")])
            .expect("replace_all should succeed");
        assert_eq!(result, s("baz bar baz"));
    }

    #[test]
    fn replace_with_capture_ref() {
        let result = HOST_REGISTRY
            .call(
                "regex_replace_all",
                &[s("John Smith"), s(r"(\w+) (\w+)"), s("$2, $1")],
            )
            .expect("replace_all should succeed");
        assert_eq!(result, s("Smith, John"));
    }

    #[test]
    fn split_by_pattern() {
        let result = HOST_REGISTRY
            .call("regex_split", &[s("one::two:::three"), s(r":+")])
            .expect("split should succeed");
        assert_eq!(
            result,
            RuntimeValue::List(vec![s("one"), s("two"), s("three")])
        );
    }

    #[test]
    fn split_by_whitespace() {
        let result = HOST_REGISTRY
            .call("regex_split", &[s("hello   world\tfoo"), s(r"\s+")])
            .expect("split should succeed");
        assert_eq!(
            result,
            RuntimeValue::List(vec![s("hello"), s("world"), s("foo")])
        );
    }

    #[test]
    fn invalid_pattern_returns_error() {
        let result = HOST_REGISTRY.call("regex_match", &[s("test"), s(r"[invalid")]);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("invalid regex pattern"),
            "error should mention invalid pattern: {}",
            err
        );
    }

    #[test]
    fn match_wrong_arity() {
        let result = HOST_REGISTRY.call("regex_match", &[s("test")]);
        assert!(result.is_err());
    }

    #[test]
    fn match_non_string_arg() {
        let result = HOST_REGISTRY.call("regex_match", &[RuntimeValue::Int(42), s(r"\d+")]);
        assert!(result.is_err());
    }

    #[test]
    fn scan_empty_matches() {
        let result = HOST_REGISTRY
            .call("regex_scan", &[s("hello"), s(r"\d+")])
            .expect("scan should succeed");
        assert_eq!(result, RuntimeValue::List(vec![]));
    }

    #[test]
    fn run_optional_capture_group() {
        let result = HOST_REGISTRY
            .call("regex_run", &[s("ac"), s(r"a(b)?c")])
            .expect("run should succeed");
        match result {
            RuntimeValue::List(caps) => {
                assert_eq!(caps.len(), 2);
                assert_eq!(caps[0], s("ac"));
                assert_eq!(caps[1], RuntimeValue::Nil); // optional group not matched
            }
            other => panic!("expected list, got {:?}", other),
        }
    }

    #[test]
    fn registration() {
        assert!(HOST_REGISTRY.call("regex_match", &[s("a"), s("a")]).is_ok());
        assert!(HOST_REGISTRY.call("regex_run", &[s("a"), s("a")]).is_ok());
        assert!(HOST_REGISTRY.call("regex_scan", &[s("a"), s("a")]).is_ok());
        assert!(HOST_REGISTRY
            .call("regex_replace", &[s("a"), s("a"), s("b")])
            .is_ok());
        assert!(HOST_REGISTRY
            .call("regex_replace_all", &[s("a"), s("a"), s("b")])
            .is_ok());
        assert!(HOST_REGISTRY.call("regex_split", &[s("a"), s("a")]).is_ok());
    }
}
