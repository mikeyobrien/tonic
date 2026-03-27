use super::system::expect_exact_args;
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn require_string<'a>(name: &str, val: &'a RuntimeValue) -> Result<&'a str, HostError> {
    match val {
        RuntimeValue::String(s) => Ok(s.as_str()),
        other => Err(HostError::new(format!(
            "{} expects a string argument, found {}",
            name,
            host_value_kind(other)
        ))),
    }
}

fn ok_tuple(val: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("ok".to_string())),
        Box::new(val),
    )
}

fn error_tuple(msg: String) -> RuntimeValue {
    RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("error".to_string())),
        Box::new(RuntimeValue::String(msg)),
    )
}

fn host_hex_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Hex.encode", args, 1)?;
    let s = require_string("Hex.encode", &args[0])?;
    let hex: String = s.bytes().map(|b| format!("{:02x}", b)).collect();
    Ok(RuntimeValue::String(hex))
}

fn host_hex_encode_upper(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Hex.encode_upper", args, 1)?;
    let s = require_string("Hex.encode_upper", &args[0])?;
    let hex: String = s.bytes().map(|b| format!("{:02X}", b)).collect();
    Ok(RuntimeValue::String(hex))
}

fn host_hex_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Hex.decode", args, 1)?;
    let s = require_string("Hex.decode", &args[0])?;

    if s.len() % 2 != 0 {
        return Ok(error_tuple("odd-length hex string".to_string()));
    }

    let mut bytes = Vec::with_capacity(s.len() / 2);
    let chars: Vec<u8> = s.bytes().collect();
    let mut i = 0;
    while i < chars.len() {
        let hi = hex_nibble(chars[i]);
        let lo = hex_nibble(chars[i + 1]);
        match (hi, lo) {
            (Some(h), Some(l)) => bytes.push((h << 4) | l),
            _ => {
                return Ok(error_tuple(format!(
                    "invalid hex character at position {}",
                    if hi.is_none() { i } else { i + 1 }
                )));
            }
        }
        i += 2;
    }

    match String::from_utf8(bytes) {
        Ok(decoded) => Ok(ok_tuple(RuntimeValue::String(decoded))),
        Err(e) => {
            let decoded = String::from_utf8_lossy(e.as_bytes()).into_owned();
            Ok(ok_tuple(RuntimeValue::String(decoded)))
        }
    }
}

fn hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

pub fn register_hex_host_functions(registry: &HostRegistry) {
    registry.register("hex_encode", host_hex_encode);
    registry.register("hex_encode_upper", host_hex_encode_upper);
    registry.register("hex_decode", host_hex_decode);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_val(s: &str) -> RuntimeValue {
        ok_tuple(RuntimeValue::String(s.to_string()))
    }

    fn err_val(s: &str) -> RuntimeValue {
        error_tuple(s.to_string())
    }

    #[test]
    fn encode_empty_string() {
        let result = host_hex_encode(&[RuntimeValue::String("".to_string())]).unwrap();
        assert_eq!(result, RuntimeValue::String("".to_string()));
    }

    #[test]
    fn encode_ascii() {
        let result = host_hex_encode(&[RuntimeValue::String("hello".to_string())]).unwrap();
        assert_eq!(result, RuntimeValue::String("68656c6c6f".to_string()));
    }

    #[test]
    fn encode_binary_bytes() {
        let result = host_hex_encode(&[RuntimeValue::String("\x00\x09\x7f".to_string())]).unwrap();
        assert_eq!(result, RuntimeValue::String("00097f".to_string()));
    }

    #[test]
    fn encode_upper_case() {
        let result = host_hex_encode_upper(&[RuntimeValue::String("hello".to_string())]).unwrap();
        assert_eq!(result, RuntimeValue::String("68656C6C6F".to_string()));
    }

    #[test]
    fn decode_valid_hex() {
        let result = host_hex_decode(&[RuntimeValue::String("68656c6c6f".to_string())]).unwrap();
        assert_eq!(result, ok_val("hello"));
    }

    #[test]
    fn decode_uppercase_hex() {
        let result = host_hex_decode(&[RuntimeValue::String("68656C6C6F".to_string())]).unwrap();
        assert_eq!(result, ok_val("hello"));
    }

    #[test]
    fn decode_round_trip() {
        let original = "Hello, World!";
        let encoded = host_hex_encode(&[RuntimeValue::String(original.to_string())]).unwrap();
        if let RuntimeValue::String(ref hex) = encoded {
            let decoded = host_hex_decode(&[RuntimeValue::String(hex.clone())]).unwrap();
            assert_eq!(decoded, ok_val(original));
        } else {
            panic!("expected string");
        }
    }

    #[test]
    fn decode_odd_length_error() {
        let result = host_hex_decode(&[RuntimeValue::String("abc".to_string())]).unwrap();
        assert_eq!(result, err_val("odd-length hex string"));
    }

    #[test]
    fn decode_invalid_char_error() {
        let result = host_hex_decode(&[RuntimeValue::String("zz".to_string())]).unwrap();
        if let RuntimeValue::Tuple(ref first, _) = result {
            assert_eq!(**first, RuntimeValue::Atom("error".to_string()));
        } else {
            panic!("expected tuple");
        }
    }

    #[test]
    fn encode_rejects_non_string() {
        let result = host_hex_encode(&[RuntimeValue::Int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn encode_rejects_wrong_arity() {
        let result = host_hex_encode(&[
            RuntimeValue::String("a".to_string()),
            RuntimeValue::String("b".to_string()),
        ]);
        assert!(result.is_err());
    }

    #[test]
    fn registration_via_call() {
        let registry = HostRegistry::new();
        register_hex_host_functions(&registry);
        let result = registry
            .call("hex_encode", &[RuntimeValue::String("A".to_string())])
            .unwrap();
        assert_eq!(result, RuntimeValue::String("41".to_string()));
    }
}
