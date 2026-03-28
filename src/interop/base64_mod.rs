use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;

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

fn host_base64_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Base64.encode", args, 1)?;
    let s = extract_string("Base64.encode", args)?;
    Ok(RuntimeValue::String(STANDARD.encode(s.as_bytes())))
}

fn host_base64_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Base64.decode", args, 1)?;
    let s = extract_string("Base64.decode", args)?;
    let bytes = STANDARD
        .decode(s.as_bytes())
        .map_err(|e| HostError::new(format!("Base64.decode: invalid base64 input: {}", e)))?;
    let decoded = String::from_utf8(bytes).map_err(|e| {
        HostError::new(format!(
            "Base64.decode: decoded bytes are not valid UTF-8: {}",
            e
        ))
    })?;
    Ok(RuntimeValue::String(decoded))
}

fn host_base64_url_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Base64.url_encode", args, 1)?;
    let s = extract_string("Base64.url_encode", args)?;
    Ok(RuntimeValue::String(URL_SAFE_NO_PAD.encode(s.as_bytes())))
}

fn host_base64_url_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Base64.url_decode", args, 1)?;
    let s = extract_string("Base64.url_decode", args)?;
    let bytes = URL_SAFE_NO_PAD.decode(s.as_bytes()).map_err(|e| {
        HostError::new(format!("Base64.url_decode: invalid base64url input: {}", e))
    })?;
    let decoded = String::from_utf8(bytes).map_err(|e| {
        HostError::new(format!(
            "Base64.url_decode: decoded bytes are not valid UTF-8: {}",
            e
        ))
    })?;
    Ok(RuntimeValue::String(decoded))
}

pub fn register_base64_host_functions(registry: &HostRegistry) {
    registry.register("base64_encode", host_base64_encode);
    registry.register("base64_decode", host_base64_decode);
    registry.register("base64_url_encode", host_base64_url_encode);
    registry.register("base64_url_decode", host_base64_url_decode);
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
            .call("base64_encode", &[s("hello")])
            .expect("encode should succeed");
        assert_eq!(result, s("aGVsbG8="));
    }

    #[test]
    fn decode_simple_string() {
        let result = HOST_REGISTRY
            .call("base64_decode", &[s("aGVsbG8=")])
            .expect("decode should succeed");
        assert_eq!(result, s("hello"));
    }

    #[test]
    fn encode_decode_round_trip() {
        let original = s("The quick brown fox jumps over the lazy dog");
        let encoded = HOST_REGISTRY
            .call("base64_encode", std::slice::from_ref(&original))
            .expect("encode should succeed");
        let decoded = HOST_REGISTRY
            .call("base64_decode", &[encoded])
            .expect("decode should succeed");
        assert_eq!(decoded, original);
    }

    #[test]
    fn encode_empty_string() {
        let result = HOST_REGISTRY
            .call("base64_encode", &[s("")])
            .expect("encode should succeed");
        assert_eq!(result, s(""));
    }

    #[test]
    fn decode_empty_string() {
        let result = HOST_REGISTRY
            .call("base64_decode", &[s("")])
            .expect("decode should succeed");
        assert_eq!(result, s(""));
    }

    #[test]
    fn decode_invalid_input_returns_error() {
        let err = HOST_REGISTRY
            .call("base64_decode", &[s("not valid base64!!!")])
            .expect_err("decode should fail on invalid input");
        assert!(
            err.to_string().contains("invalid base64"),
            "error should mention invalid base64: {err}"
        );
    }

    #[test]
    fn url_encode_simple_string() {
        let result = HOST_REGISTRY
            .call("base64_url_encode", &[s("hello")])
            .expect("url_encode should succeed");
        // URL-safe no-pad: aGVsbG8 (no trailing =)
        assert_eq!(result, s("aGVsbG8"));
    }

    #[test]
    fn url_decode_simple_string() {
        let result = HOST_REGISTRY
            .call("base64_url_decode", &[s("aGVsbG8")])
            .expect("url_decode should succeed");
        assert_eq!(result, s("hello"));
    }

    #[test]
    fn url_encode_decode_round_trip() {
        let original = s("data with +/= chars: foo+bar/baz=qux");
        let encoded = HOST_REGISTRY
            .call("base64_url_encode", std::slice::from_ref(&original))
            .expect("url_encode should succeed");
        // URL-safe should not contain + or /
        if let RuntimeValue::String(ref enc) = encoded {
            assert!(!enc.contains('+'), "URL-safe should not contain +");
            assert!(!enc.contains('/'), "URL-safe should not contain /");
        }
        let decoded = HOST_REGISTRY
            .call("base64_url_decode", &[encoded])
            .expect("url_decode should succeed");
        assert_eq!(decoded, original);
    }

    #[test]
    fn encode_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("base64_encode", &[RuntimeValue::Int(42)])
            .expect_err("encode should fail on non-string");
        assert!(
            err.to_string().contains("string"),
            "error should mention string: {err}"
        );
    }

    #[test]
    fn url_decode_invalid_input_returns_error() {
        let err = HOST_REGISTRY
            .call("base64_url_decode", &[s("not valid!!!")])
            .expect_err("url_decode should fail on invalid input");
        assert!(
            err.to_string().contains("invalid base64url"),
            "error should mention invalid base64url: {err}"
        );
    }

    #[test]
    fn encode_with_padding() {
        // "a" -> "YQ==" (standard padding)
        let result = HOST_REGISTRY
            .call("base64_encode", &[s("a")])
            .expect("encode should succeed");
        assert_eq!(result, s("YQ=="));
    }

    #[test]
    fn url_encode_no_padding() {
        // "a" -> "YQ" (no padding in URL-safe)
        let result = HOST_REGISTRY
            .call("base64_url_encode", &[s("a")])
            .expect("url_encode should succeed");
        assert_eq!(result, s("YQ"));
    }
}
