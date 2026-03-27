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

fn extract_string<'a>(function: &str, args: &'a [RuntimeValue]) -> Result<&'a str, HostError> {
    match &args[0] {
        RuntimeValue::String(s) => Ok(s.as_str()),
        other => Err(HostError::new(format!(
            "{} expects a string argument, found {}",
            function,
            host_value_kind(other)
        ))),
    }
}

/// RFC 3986 unreserved characters: ALPHA / DIGIT / "-" / "." / "_" / "~"
fn is_unreserved(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_' || b == b'~'
}

fn percent_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        if is_unreserved(*byte) {
            encoded.push(*byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

fn percent_decode(input: &str) -> Result<String, HostError> {
    let bytes = input.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return Err(HostError::new(format!(
                    "Url.decode: incomplete percent-encoding at position {}",
                    i
                )));
            }
            let hi = bytes[i + 1];
            let lo = bytes[i + 2];
            let hex_bytes = [hi, lo];
            let hex_str = std::str::from_utf8(&hex_bytes).map_err(|_| {
                HostError::new(format!(
                    "Url.decode: invalid percent-encoding at position {}",
                    i
                ))
            })?;
            let byte = u8::from_str_radix(hex_str, 16).map_err(|_| {
                HostError::new(format!(
                    "Url.decode: invalid hex digits '{}' at position {}",
                    hex_str, i
                ))
            })?;
            decoded.push(byte);
            i += 3;
        } else if bytes[i] == b'+' {
            decoded.push(b' ');
            i += 1;
        } else {
            decoded.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(decoded).map_err(|e| {
        HostError::new(format!(
            "Url.decode: decoded bytes are not valid UTF-8: {}",
            e
        ))
    })
}

/// Encode for query string values (space → +, other specials → %XX)
fn query_encode_component(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.as_bytes() {
        if byte.is_ascii_alphanumeric() || *byte == b'-' || *byte == b'_' || *byte == b'.' {
            encoded.push(*byte as char);
        } else if *byte == b' ' {
            encoded.push('+');
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }
    encoded
}

fn host_url_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Url.encode", args, 1)?;
    let s = extract_string("Url.encode", args)?;
    Ok(RuntimeValue::String(percent_encode(s)))
}

fn host_url_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Url.decode", args, 1)?;
    let s = extract_string("Url.decode", args)?;
    percent_decode(s).map(RuntimeValue::String)
}

fn host_url_encode_query(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Url.encode_query", args, 1)?;
    match &args[0] {
        RuntimeValue::Map(entries) => {
            let mut parts = Vec::new();
            for (k, v) in entries {
                let key_str = value_to_string(k);
                let val_str = value_to_string(v);
                parts.push(format!(
                    "{}={}",
                    query_encode_component(&key_str),
                    query_encode_component(&val_str)
                ));
            }
            Ok(RuntimeValue::String(parts.join("&")))
        }
        RuntimeValue::List(items) => {
            // Expect list of {key, value} tuples (keyword list style)
            let mut parts = Vec::new();
            for item in items {
                match item {
                    RuntimeValue::Tuple(k, v) => {
                        let key_str = value_to_string(k);
                        let val_str = value_to_string(v);
                        parts.push(format!(
                            "{}={}",
                            query_encode_component(&key_str),
                            query_encode_component(&val_str)
                        ));
                    }
                    other => {
                        return Err(HostError::new(format!(
                            "Url.encode_query: list elements must be {{key, value}} tuples, found {}",
                            host_value_kind(other)
                        )));
                    }
                }
            }
            Ok(RuntimeValue::String(parts.join("&")))
        }
        other => Err(HostError::new(format!(
            "Url.encode_query expects a map or keyword list, found {}",
            host_value_kind(other)
        ))),
    }
}

fn host_url_decode_query(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Url.decode_query", args, 1)?;
    let s = extract_string("Url.decode_query", args)?;
    if s.is_empty() {
        return Ok(RuntimeValue::Map(Vec::new()));
    }
    let mut entries = Vec::new();
    for pair in s.split('&') {
        let (key_raw, val_raw) = match pair.split_once('=') {
            Some((k, v)) => (k, v),
            None => (pair, ""),
        };
        let key = percent_decode(key_raw)?;
        let val = percent_decode(val_raw)?;
        entries.push((RuntimeValue::String(key), RuntimeValue::String(val)));
    }
    Ok(RuntimeValue::Map(entries))
}

fn value_to_string(v: &RuntimeValue) -> String {
    match v {
        RuntimeValue::String(s) => s.clone(),
        RuntimeValue::Int(n) => n.to_string(),
        RuntimeValue::Float(f) => f.to_string(),
        RuntimeValue::Bool(b) => b.to_string(),
        RuntimeValue::Atom(a) => a.clone(),
        RuntimeValue::Nil => "".to_string(),
        _ => format!("{:?}", v),
    }
}

pub fn register_url_host_functions(registry: &HostRegistry) {
    registry.register("url_encode", host_url_encode);
    registry.register("url_decode", host_url_decode);
    registry.register("url_encode_query", host_url_encode_query);
    registry.register("url_decode_query", host_url_decode_query);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn encode_simple_string() {
        let result = HOST_REGISTRY
            .call("url_encode", &[s("hello world")])
            .expect("encode should succeed");
        assert_eq!(result, s("hello%20world"));
    }

    #[test]
    fn encode_preserves_unreserved() {
        let result = HOST_REGISTRY
            .call("url_encode", &[s("hello-world_2.0~test")])
            .expect("encode should succeed");
        assert_eq!(result, s("hello-world_2.0~test"));
    }

    #[test]
    fn encode_special_characters() {
        let result = HOST_REGISTRY
            .call("url_encode", &[s("a=1&b=2")])
            .expect("encode should succeed");
        assert_eq!(result, s("a%3D1%26b%3D2"));
    }

    #[test]
    fn encode_unicode() {
        let result = HOST_REGISTRY
            .call("url_encode", &[s("café")])
            .expect("encode should succeed");
        assert_eq!(result, s("caf%C3%A9"));
    }

    #[test]
    fn decode_simple_string() {
        let result = HOST_REGISTRY
            .call("url_decode", &[s("hello%20world")])
            .expect("decode should succeed");
        assert_eq!(result, s("hello world"));
    }

    #[test]
    fn decode_plus_as_space() {
        let result = HOST_REGISTRY
            .call("url_decode", &[s("hello+world")])
            .expect("decode should succeed");
        assert_eq!(result, s("hello world"));
    }

    #[test]
    fn encode_decode_round_trip() {
        let original = s("hello world & café = good!");
        let encoded = HOST_REGISTRY
            .call("url_encode", &[original.clone()])
            .expect("encode should succeed");
        let decoded = HOST_REGISTRY
            .call("url_decode", &[encoded])
            .expect("decode should succeed");
        assert_eq!(decoded, original);
    }

    #[test]
    fn encode_empty_string() {
        let result = HOST_REGISTRY
            .call("url_encode", &[s("")])
            .expect("encode should succeed");
        assert_eq!(result, s(""));
    }

    #[test]
    fn decode_invalid_percent_encoding() {
        let err = HOST_REGISTRY
            .call("url_decode", &[s("hello%GZ")])
            .expect_err("decode should fail on invalid hex");
        assert!(
            err.to_string().contains("invalid hex"),
            "error should mention invalid hex: {err}"
        );
    }

    #[test]
    fn decode_incomplete_percent_encoding() {
        let err = HOST_REGISTRY
            .call("url_decode", &[s("hello%2")])
            .expect_err("decode should fail on incomplete encoding");
        assert!(
            err.to_string().contains("incomplete"),
            "error should mention incomplete: {err}"
        );
    }

    #[test]
    fn encode_query_from_map() {
        let map = RuntimeValue::Map(vec![
            (s("name"), s("John Doe")),
            (s("age"), RuntimeValue::Int(30)),
        ]);
        let result = HOST_REGISTRY
            .call("url_encode_query", &[map])
            .expect("encode_query should succeed");
        if let RuntimeValue::String(ref qs) = result {
            assert!(qs.contains("name=John+Doe"), "should encode name: {qs}");
            assert!(qs.contains("age=30"), "should encode age: {qs}");
            assert!(qs.contains('&'), "should join with &: {qs}");
        } else {
            panic!("expected string result");
        }
    }

    #[test]
    fn encode_query_from_tuple_list() {
        let list = RuntimeValue::List(vec![
            RuntimeValue::Tuple(Box::new(s("q")), Box::new(s("rust lang"))),
            RuntimeValue::Tuple(Box::new(s("page")), Box::new(RuntimeValue::Int(1))),
        ]);
        let result = HOST_REGISTRY
            .call("url_encode_query", &[list])
            .expect("encode_query should succeed");
        assert_eq!(result, s("q=rust+lang&page=1"));
    }

    #[test]
    fn decode_query_string() {
        let result = HOST_REGISTRY
            .call("url_decode_query", &[s("name=John+Doe&age=30")])
            .expect("decode_query should succeed");
        if let RuntimeValue::Map(entries) = result {
            assert_eq!(entries.len(), 2);
            assert!(entries.contains(&(s("name"), s("John Doe"))));
            assert!(entries.contains(&(s("age"), s("30"))));
        } else {
            panic!("expected map result");
        }
    }

    #[test]
    fn decode_query_empty_string() {
        let result = HOST_REGISTRY
            .call("url_decode_query", &[s("")])
            .expect("decode_query should succeed");
        assert_eq!(result, RuntimeValue::Map(vec![]));
    }

    #[test]
    fn decode_query_key_without_value() {
        let result = HOST_REGISTRY
            .call("url_decode_query", &[s("key1&key2=val")])
            .expect("decode_query should succeed");
        if let RuntimeValue::Map(entries) = result {
            assert_eq!(entries.len(), 2);
            assert!(entries.contains(&(s("key1"), s(""))));
            assert!(entries.contains(&(s("key2"), s("val"))));
        } else {
            panic!("expected map result");
        }
    }

    #[test]
    fn encode_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("url_encode", &[RuntimeValue::Int(42)])
            .expect_err("encode should fail on non-string");
        assert!(
            err.to_string().contains("string"),
            "error should mention string: {err}"
        );
    }

    #[test]
    fn encode_query_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("url_encode_query", &[s("not a map")])
            .expect_err("encode_query should fail on string input");
        assert!(
            err.to_string().contains("map"),
            "error should mention map: {err}"
        );
    }
}
