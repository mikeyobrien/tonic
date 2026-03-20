use super::system::{expect_exact_args, expect_int_arg, expect_string_arg};
use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn host_string_split(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.split", args, 2)?;
    let s = expect_string_arg("String.split", args, 0)?;
    let delimiter = expect_string_arg("String.split", args, 1)?;
    let parts: Vec<RuntimeValue> = s
        .split(delimiter.as_str())
        .map(|part| RuntimeValue::String(part.to_string()))
        .collect();
    Ok(RuntimeValue::List(parts))
}

fn host_string_replace(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.replace", args, 3)?;
    let s = expect_string_arg("String.replace", args, 0)?;
    let pattern = expect_string_arg("String.replace", args, 1)?;
    let replacement = expect_string_arg("String.replace", args, 2)?;
    Ok(RuntimeValue::String(
        s.replace(pattern.as_str(), &replacement),
    ))
}

fn host_string_trim(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.trim", args, 1)?;
    let s = expect_string_arg("String.trim", args, 0)?;
    Ok(RuntimeValue::String(s.trim().to_string()))
}

fn host_string_trim_leading(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.trim_leading", args, 1)?;
    let s = expect_string_arg("String.trim_leading", args, 0)?;
    Ok(RuntimeValue::String(s.trim_start().to_string()))
}

fn host_string_trim_trailing(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.trim_trailing", args, 1)?;
    let s = expect_string_arg("String.trim_trailing", args, 0)?;
    Ok(RuntimeValue::String(s.trim_end().to_string()))
}

fn host_string_starts_with(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.starts_with?", args, 2)?;
    let s = expect_string_arg("String.starts_with?", args, 0)?;
    let prefix = expect_string_arg("String.starts_with?", args, 1)?;
    Ok(RuntimeValue::Bool(s.starts_with(prefix.as_str())))
}

fn host_string_ends_with(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.ends_with?", args, 2)?;
    let s = expect_string_arg("String.ends_with?", args, 0)?;
    let suffix = expect_string_arg("String.ends_with?", args, 1)?;
    Ok(RuntimeValue::Bool(s.ends_with(suffix.as_str())))
}

fn host_string_contains(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.contains?", args, 2)?;
    let s = expect_string_arg("String.contains?", args, 0)?;
    let substr = expect_string_arg("String.contains?", args, 1)?;
    Ok(RuntimeValue::Bool(s.contains(substr.as_str())))
}

fn host_string_upcase(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.upcase", args, 1)?;
    let s = expect_string_arg("String.upcase", args, 0)?;
    Ok(RuntimeValue::String(s.to_uppercase()))
}

fn host_string_downcase(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.downcase", args, 1)?;
    let s = expect_string_arg("String.downcase", args, 0)?;
    Ok(RuntimeValue::String(s.to_lowercase()))
}

fn host_string_length(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.length", args, 1)?;
    let s = expect_string_arg("String.length", args, 0)?;
    Ok(RuntimeValue::Int(s.chars().count() as i64))
}

fn host_string_to_charlist(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.to_charlist", args, 1)?;
    let s = expect_string_arg("String.to_charlist", args, 0)?;
    Ok(RuntimeValue::List(
        s.chars()
            .map(|ch| RuntimeValue::Int(i64::from(u32::from(ch))))
            .collect(),
    ))
}

fn host_string_at(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.at", args, 2)?;
    let s = expect_string_arg("String.at", args, 0)?;
    let index = expect_int_arg("String.at", args, 1)?;
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len() as i64;

    // Support negative indexing like Elixir
    let resolved = if index < 0 { len + index } else { index };

    if resolved < 0 || resolved >= len {
        return Ok(RuntimeValue::Nil);
    }

    Ok(RuntimeValue::String(chars[resolved as usize].to_string()))
}

fn host_string_slice(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.slice", args, 3)?;
    let s = expect_string_arg("String.slice", args, 0)?;
    let start = expect_int_arg("String.slice", args, 1)?;
    let len = expect_int_arg("String.slice", args, 2)?;
    let chars: Vec<char> = s.chars().collect();
    let char_count = chars.len() as i64;

    let resolved_start = if start < 0 {
        (char_count + start).max(0)
    } else {
        start.min(char_count)
    } as usize;

    let resolved_len = len.max(0) as usize;
    let end = (resolved_start + resolved_len).min(chars.len());
    let slice: String = chars[resolved_start..end].iter().collect();
    Ok(RuntimeValue::String(slice))
}

fn host_string_to_integer(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.to_integer", args, 1)?;
    let s = expect_string_arg("String.to_integer", args, 0)?;
    match s.trim().parse::<i64>() {
        Ok(n) => Ok(RuntimeValue::Int(n)),
        Err(_) => Err(HostError::new(format!(
            "String.to_integer could not parse {:?} as integer",
            s
        ))),
    }
}

fn host_string_to_float(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.to_float", args, 1)?;
    let s = expect_string_arg("String.to_float", args, 0)?;
    match s.trim().parse::<f64>() {
        Ok(f) => Ok(RuntimeValue::Float(f.to_string())),
        Err(_) => Err(HostError::new(format!(
            "String.to_float could not parse {:?} as float",
            s
        ))),
    }
}

fn pad_string(
    name: &str,
    s: String,
    count: i64,
    padding: &str,
    leading: bool,
) -> Result<RuntimeValue, HostError> {
    if count < 0 {
        return Err(HostError::new(format!("{name} count must be non-negative")));
    }
    if padding.is_empty() {
        return Err(HostError::new(format!("{name} padding must not be empty")));
    }
    let target = count as usize;
    let current_len = s.chars().count();
    if current_len >= target {
        return Ok(RuntimeValue::String(s));
    }
    let needed = target - current_len;
    let pad_chars: Vec<char> = padding.chars().collect();
    let pad: String = (0..needed)
        .map(|i| pad_chars[i % pad_chars.len()])
        .collect();
    if leading {
        Ok(RuntimeValue::String(format!("{pad}{s}")))
    } else {
        Ok(RuntimeValue::String(format!("{s}{pad}")))
    }
}

fn host_string_pad_leading(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.pad_leading", args, 3)?;
    let s = expect_string_arg("String.pad_leading", args, 0)?;
    let count = expect_int_arg("String.pad_leading", args, 1)?;
    let padding = expect_string_arg("String.pad_leading", args, 2)?;
    pad_string("String.pad_leading", s, count, &padding, true)
}

fn host_string_pad_trailing(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.pad_trailing", args, 3)?;
    let s = expect_string_arg("String.pad_trailing", args, 0)?;
    let count = expect_int_arg("String.pad_trailing", args, 1)?;
    let padding = expect_string_arg("String.pad_trailing", args, 2)?;
    pad_string("String.pad_trailing", s, count, &padding, false)
}

fn host_string_reverse(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.reverse", args, 1)?;
    let s = expect_string_arg("String.reverse", args, 0)?;
    Ok(RuntimeValue::String(s.chars().rev().collect()))
}

fn host_string_to_atom(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.to_atom", args, 1)?;
    let s = expect_string_arg("String.to_atom", args, 0)?;
    Ok(RuntimeValue::Atom(s))
}

fn host_string_graphemes(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("String.graphemes", args, 1)?;
    let s = expect_string_arg("String.graphemes", args, 0)?;
    let graphemes: Vec<RuntimeValue> = s
        .chars()
        .map(|ch| RuntimeValue::String(ch.to_string()))
        .collect();
    Ok(RuntimeValue::List(graphemes))
}

pub fn register_string_host_functions(registry: &HostRegistry) {
    registry.register("str_split", host_string_split);
    registry.register("str_replace", host_string_replace);
    registry.register("str_trim", host_string_trim);
    registry.register("str_trim_leading", host_string_trim_leading);
    registry.register("str_trim_trailing", host_string_trim_trailing);
    registry.register("str_starts_with", host_string_starts_with);
    registry.register("str_ends_with", host_string_ends_with);
    registry.register("str_contains", host_string_contains);
    registry.register("str_upcase", host_string_upcase);
    registry.register("str_downcase", host_string_downcase);
    registry.register("str_length", host_string_length);
    registry.register("str_to_charlist", host_string_to_charlist);
    registry.register("str_at", host_string_at);
    registry.register("str_slice", host_string_slice);
    registry.register("str_to_integer", host_string_to_integer);
    registry.register("str_to_float", host_string_to_float);
    registry.register("str_pad_leading", host_string_pad_leading);
    registry.register("str_pad_trailing", host_string_pad_trailing);
    registry.register("str_reverse", host_string_reverse);
    registry.register("str_to_atom", host_string_to_atom);
    registry.register("str_graphemes", host_string_graphemes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interop::{HostRegistry, HOST_REGISTRY};

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    fn i(n: i64) -> RuntimeValue {
        RuntimeValue::Int(n)
    }

    #[test]
    fn str_split_splits_by_delimiter() {
        let result = HOST_REGISTRY
            .call("str_split", &[s("a,b,c"), s(",")])
            .expect("str_split should succeed");
        assert_eq!(result, RuntimeValue::List(vec![s("a"), s("b"), s("c")]));
    }

    #[test]
    fn str_replace_replaces_pattern() {
        let result = HOST_REGISTRY
            .call("str_replace", &[s("hello world"), s("world"), s("Tonic")])
            .expect("str_replace should succeed");
        assert_eq!(result, s("hello Tonic"));
    }

    #[test]
    fn str_trim_removes_whitespace() {
        let result = HOST_REGISTRY
            .call("str_trim", &[s("  hello  ")])
            .expect("str_trim should succeed");
        assert_eq!(result, s("hello"));
    }

    #[test]
    fn str_trim_leading_removes_leading_whitespace() {
        let result = HOST_REGISTRY
            .call("str_trim_leading", &[s("  hello  ")])
            .expect("str_trim_leading should succeed");
        assert_eq!(result, s("hello  "));
    }

    #[test]
    fn str_trim_trailing_removes_trailing_whitespace() {
        let result = HOST_REGISTRY
            .call("str_trim_trailing", &[s("  hello  ")])
            .expect("str_trim_trailing should succeed");
        assert_eq!(result, s("  hello"));
    }

    #[test]
    fn str_starts_with_detects_prefix() {
        let yes = HOST_REGISTRY
            .call("str_starts_with", &[s("hello"), s("he")])
            .expect("str_starts_with should succeed");
        assert_eq!(yes, RuntimeValue::Bool(true));

        let no = HOST_REGISTRY
            .call("str_starts_with", &[s("hello"), s("lo")])
            .expect("str_starts_with should succeed");
        assert_eq!(no, RuntimeValue::Bool(false));
    }

    #[test]
    fn str_ends_with_detects_suffix() {
        let yes = HOST_REGISTRY
            .call("str_ends_with", &[s("hello"), s("lo")])
            .expect("str_ends_with should succeed");
        assert_eq!(yes, RuntimeValue::Bool(true));
    }

    #[test]
    fn str_contains_detects_substring() {
        let yes = HOST_REGISTRY
            .call("str_contains", &[s("hello world"), s("lo w")])
            .expect("str_contains should succeed");
        assert_eq!(yes, RuntimeValue::Bool(true));
    }

    #[test]
    fn str_upcase_uppercases_string() {
        let result = HOST_REGISTRY
            .call("str_upcase", &[s("hello")])
            .expect("str_upcase should succeed");
        assert_eq!(result, s("HELLO"));
    }

    #[test]
    fn str_downcase_lowercases_string() {
        let result = HOST_REGISTRY
            .call("str_downcase", &[s("HELLO")])
            .expect("str_downcase should succeed");
        assert_eq!(result, s("hello"));
    }

    #[test]
    fn str_length_counts_unicode_chars() {
        let result = HOST_REGISTRY
            .call("str_length", &[s("hello")])
            .expect("str_length should succeed");
        assert_eq!(result, i(5));
    }

    #[test]
    fn str_to_charlist_returns_empty_list_for_empty_string() {
        let result = HOST_REGISTRY
            .call("str_to_charlist", &[s("")])
            .expect("str_to_charlist should succeed");
        assert_eq!(result, RuntimeValue::List(vec![]));
    }

    #[test]
    fn str_to_charlist_returns_ascii_codepoints() {
        let result = HOST_REGISTRY
            .call("str_to_charlist", &[s("tonic")])
            .expect("str_to_charlist should succeed");
        assert_eq!(
            result,
            RuntimeValue::List(vec![i(116), i(111), i(110), i(105), i(99)])
        );
    }

    #[test]
    fn str_to_charlist_returns_unicode_codepoints_not_utf8_bytes() {
        let result = HOST_REGISTRY
            .call("str_to_charlist", &[s("hé🙂")])
            .expect("str_to_charlist should succeed");
        assert_eq!(result, RuntimeValue::List(vec![i(104), i(233), i(128578)]));
    }

    #[test]
    fn str_at_returns_char_at_index() {
        let result = HOST_REGISTRY
            .call("str_at", &[s("hello"), i(1)])
            .expect("str_at should succeed");
        assert_eq!(result, s("e"));
    }

    #[test]
    fn str_at_returns_nil_for_out_of_bounds() {
        let result = HOST_REGISTRY
            .call("str_at", &[s("hello"), i(100)])
            .expect("str_at should succeed for out of bounds");
        assert_eq!(result, RuntimeValue::Nil);
    }

    #[test]
    fn str_slice_returns_substring() {
        let result = HOST_REGISTRY
            .call("str_slice", &[s("hello"), i(1), i(3)])
            .expect("str_slice should succeed");
        assert_eq!(result, s("ell"));
    }

    #[test]
    fn str_to_integer_parses_integer() {
        let result = HOST_REGISTRY
            .call("str_to_integer", &[s("42")])
            .expect("str_to_integer should succeed");
        assert_eq!(result, i(42));
    }

    #[test]
    fn str_to_integer_rejects_non_integer() {
        let error = HOST_REGISTRY
            .call("str_to_integer", &[s("abc")])
            .expect_err("str_to_integer should fail for non-integer");
        assert!(error.to_string().contains("String.to_integer"));
    }

    #[test]
    fn str_to_float_parses_float() {
        let result = HOST_REGISTRY
            .call("str_to_float", &[s("3.14")])
            .expect("str_to_float should succeed");
        assert_eq!(result, s("3.14"));
    }

    #[test]
    fn str_pad_leading_pads_string() {
        let result = HOST_REGISTRY
            .call("str_pad_leading", &[s("hi"), i(5), s(" ")])
            .expect("str_pad_leading should succeed");
        assert_eq!(result, s("   hi"));
    }

    #[test]
    fn str_pad_trailing_pads_string() {
        let result = HOST_REGISTRY
            .call("str_pad_trailing", &[s("hi"), i(5), s(" ")])
            .expect("str_pad_trailing should succeed");
        assert_eq!(result, s("hi   "));
    }

    #[test]
    fn str_reverse_reverses_string() {
        let result = HOST_REGISTRY
            .call("str_reverse", &[s("hello")])
            .expect("str_reverse should succeed");
        assert_eq!(result, s("olleh"));
    }

    #[test]
    fn str_split_rejects_wrong_arity() {
        let error = HOST_REGISTRY
            .call("str_split", &[s("hello")])
            .expect_err("str_split should reject wrong arity");
        assert_eq!(
            error.to_string(),
            "host error: String.split expects exactly 2 arguments, found 1"
        );
    }

    // Suppress unused import warning for single-module test registry
    #[allow(dead_code)]
    fn _use_local_registry(_r: &HostRegistry) {}
}
