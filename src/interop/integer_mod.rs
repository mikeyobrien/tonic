use super::system::expect_exact_args;
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn require_int(name: &str, val: &RuntimeValue) -> Result<i64, HostError> {
    match val {
        RuntimeValue::Int(n) => Ok(*n),
        other => Err(HostError::new(format!(
            "{} expects integer argument, found {}",
            name,
            host_value_kind(other)
        ))),
    }
}

fn host_integer_to_string(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.to_string", args, 1)?;
    let n = require_int("Integer.to_string", &args[0])?;
    Ok(RuntimeValue::String(n.to_string()))
}

/// Convert integer to string in a given base (2..36).
fn host_integer_to_string_base(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.to_string_base", args, 2)?;
    let n = require_int("Integer.to_string_base", &args[0])?;
    let base = require_int("Integer.to_string_base", &args[1])?;
    if !(2..=36).contains(&base) {
        return Err(HostError::new(format!(
            "Integer.to_string: base must be 2..36, got {}",
            base
        )));
    }
    let base = base as u32;
    let negative = n < 0;
    let mut val = if negative {
        (n as i128).unsigned_abs()
    } else {
        n as u128
    };
    if val == 0 {
        return Ok(RuntimeValue::String("0".to_string()));
    }
    let mut digits = Vec::new();
    while val > 0 {
        let d = (val % base as u128) as u8;
        digits.push(if d < 10 { b'0' + d } else { b'a' + d - 10 });
        val /= base as u128;
    }
    if negative {
        digits.push(b'-');
    }
    digits.reverse();
    Ok(RuntimeValue::String(
        String::from_utf8(digits).unwrap_or_default(),
    ))
}

fn host_integer_parse(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.parse", args, 1)?;
    match &args[0] {
        RuntimeValue::String(s) => {
            let trimmed = s.trim_start();
            if trimmed.is_empty() {
                return Ok(RuntimeValue::Atom("error".to_string()));
            }
            let mut end = 0;
            for (i, ch) in trimmed.char_indices() {
                if i == 0 && (ch == '-' || ch == '+') {
                    end = ch.len_utf8();
                    continue;
                }
                if ch.is_ascii_digit() {
                    end = i + ch.len_utf8();
                } else {
                    break;
                }
            }
            let num_part = &trimmed[..end];
            if num_part.is_empty() || num_part == "-" || num_part == "+" {
                return Ok(RuntimeValue::Atom("error".to_string()));
            }
            match num_part.parse::<i64>() {
                Ok(n) => {
                    let rest = trimmed[end..].to_string();
                    Ok(RuntimeValue::Tuple(
                        Box::new(RuntimeValue::Int(n)),
                        Box::new(RuntimeValue::String(rest)),
                    ))
                }
                Err(_) => Ok(RuntimeValue::Atom("error".to_string())),
            }
        }
        other => Err(HostError::new(format!(
            "Integer.parse expects string argument, found {}",
            host_value_kind(other)
        ))),
    }
}

/// Return list of digits of an integer in base 10.
fn host_integer_digits(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.digits", args, 1)?;
    let n = require_int("Integer.digits", &args[0])?;
    let abs = n.unsigned_abs();
    if abs == 0 {
        return Ok(RuntimeValue::List(vec![RuntimeValue::Int(0)]));
    }
    let mut digits = Vec::new();
    let mut val = abs;
    while val > 0 {
        digits.push(RuntimeValue::Int((val % 10) as i64));
        val /= 10;
    }
    digits.reverse();
    Ok(RuntimeValue::List(digits))
}

/// Convert a list of digits back to an integer.
fn host_integer_undigits(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.undigits", args, 1)?;
    let list = match &args[0] {
        RuntimeValue::List(l) => l,
        other => {
            return Err(HostError::new(format!(
                "Integer.undigits expects a list, found {}",
                host_value_kind(other)
            )));
        }
    };
    let mut result: i64 = 0;
    for item in list {
        let d = require_int("Integer.undigits", item)?;
        if !(0..=9).contains(&d) {
            return Err(HostError::new(format!(
                "Integer.undigits: digit out of range 0..9, got {}",
                d
            )));
        }
        result = result
            .checked_mul(10)
            .and_then(|r| r.checked_add(d))
            .ok_or_else(|| HostError::new("Integer.undigits: overflow".to_string()))?;
    }
    Ok(RuntimeValue::Int(result))
}

/// Greatest common divisor (Euclidean algorithm).
fn host_integer_gcd(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.gcd", args, 2)?;
    let mut a = require_int("Integer.gcd", &args[0])?.unsigned_abs();
    let mut b = require_int("Integer.gcd", &args[1])?.unsigned_abs();
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    Ok(RuntimeValue::Int(a as i64))
}

fn host_integer_is_even(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.is_even", args, 1)?;
    let n = require_int("Integer.is_even", &args[0])?;
    Ok(RuntimeValue::Bool(n % 2 == 0))
}

fn host_integer_is_odd(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.is_odd", args, 1)?;
    let n = require_int("Integer.is_odd", &args[0])?;
    Ok(RuntimeValue::Bool(n % 2 != 0))
}

/// Integer exponentiation with overflow detection.
fn host_integer_pow(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.pow", args, 2)?;
    let base = require_int("Integer.pow", &args[0])?;
    let exp = require_int("Integer.pow", &args[1])?;
    if exp < 0 {
        return Err(HostError::new(
            "Integer.pow: exponent must be non-negative".to_string(),
        ));
    }
    let mut result: i64 = 1;
    for _ in 0..exp {
        result = result
            .checked_mul(base)
            .ok_or_else(|| HostError::new("Integer.pow: overflow".to_string()))?;
    }
    Ok(RuntimeValue::Int(result))
}

pub fn register_integer_host_functions(registry: &HostRegistry) {
    registry.register("integer_to_string", host_integer_to_string);
    registry.register("integer_to_string_base", host_integer_to_string_base);
    registry.register("integer_parse", host_integer_parse);
    registry.register("integer_digits", host_integer_digits);
    registry.register("integer_undigits", host_integer_undigits);
    registry.register("integer_gcd", host_integer_gcd);
    registry.register("integer_is_even", host_integer_is_even);
    registry.register("integer_is_odd", host_integer_is_odd);
    registry.register("integer_pow", host_integer_pow);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_int() {
        let result = host_integer_parse(&[RuntimeValue::String("42".to_string())]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Int(42)),
                Box::new(RuntimeValue::String("".to_string()))
            )
        );
    }

    #[test]
    fn parse_negative() {
        let result = host_integer_parse(&[RuntimeValue::String("-7abc".to_string())]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Int(-7)),
                Box::new(RuntimeValue::String("abc".to_string()))
            )
        );
    }

    #[test]
    fn parse_non_numeric_returns_error() {
        let result = host_integer_parse(&[RuntimeValue::String("abc".to_string())]).unwrap();
        assert_eq!(result, RuntimeValue::Atom("error".to_string()));
    }

    #[test]
    fn parse_empty_returns_error() {
        let result = host_integer_parse(&[RuntimeValue::String("".to_string())]).unwrap();
        assert_eq!(result, RuntimeValue::Atom("error".to_string()));
    }

    #[test]
    fn to_string_base_2() {
        let result =
            host_integer_to_string_base(&[RuntimeValue::Int(10), RuntimeValue::Int(2)]).unwrap();
        assert_eq!(result, RuntimeValue::String("1010".to_string()));
    }

    #[test]
    fn to_string_base_16() {
        let result =
            host_integer_to_string_base(&[RuntimeValue::Int(255), RuntimeValue::Int(16)]).unwrap();
        assert_eq!(result, RuntimeValue::String("ff".to_string()));
    }

    #[test]
    fn to_string_base_10_default() {
        let result = host_integer_to_string(&[RuntimeValue::Int(42)]).unwrap();
        assert_eq!(result, RuntimeValue::String("42".to_string()));
    }

    #[test]
    fn to_string_base_negative() {
        let result =
            host_integer_to_string_base(&[RuntimeValue::Int(-255), RuntimeValue::Int(16)]).unwrap();
        assert_eq!(result, RuntimeValue::String("-ff".to_string()));
    }

    #[test]
    fn to_string_base_invalid_rejected() {
        let result = host_integer_to_string_base(&[RuntimeValue::Int(10), RuntimeValue::Int(37)]);
        assert!(result.is_err());
    }

    #[test]
    fn digits_positive() {
        let result = host_integer_digits(&[RuntimeValue::Int(1234)]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
                RuntimeValue::Int(4)
            ])
        );
    }

    #[test]
    fn digits_zero() {
        let result = host_integer_digits(&[RuntimeValue::Int(0)]).unwrap();
        assert_eq!(result, RuntimeValue::List(vec![RuntimeValue::Int(0)]));
    }

    #[test]
    fn digits_negative_uses_abs() {
        let result = host_integer_digits(&[RuntimeValue::Int(-42)]).unwrap();
        assert_eq!(
            result,
            RuntimeValue::List(vec![RuntimeValue::Int(4), RuntimeValue::Int(2)])
        );
    }

    #[test]
    fn undigits_round_trip() {
        let digits = host_integer_digits(&[RuntimeValue::Int(1234)]).unwrap();
        let result = host_integer_undigits(&[digits]).unwrap();
        assert_eq!(result, RuntimeValue::Int(1234));
    }

    #[test]
    fn undigits_rejects_out_of_range() {
        let result = host_integer_undigits(&[RuntimeValue::List(vec![RuntimeValue::Int(10)])]);
        assert!(result.is_err());
    }

    #[test]
    fn gcd_coprime() {
        let result = host_integer_gcd(&[RuntimeValue::Int(7), RuntimeValue::Int(13)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(1));
    }

    #[test]
    fn gcd_common() {
        let result = host_integer_gcd(&[RuntimeValue::Int(12), RuntimeValue::Int(8)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(4));
    }

    #[test]
    fn gcd_with_zero() {
        let result = host_integer_gcd(&[RuntimeValue::Int(5), RuntimeValue::Int(0)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(5));
    }

    #[test]
    fn gcd_negative_uses_abs() {
        let result = host_integer_gcd(&[RuntimeValue::Int(-12), RuntimeValue::Int(8)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(4));
    }

    #[test]
    fn is_even_true() {
        let result = host_integer_is_even(&[RuntimeValue::Int(4)]).unwrap();
        assert_eq!(result, RuntimeValue::Bool(true));
    }

    #[test]
    fn is_even_false() {
        let result = host_integer_is_even(&[RuntimeValue::Int(3)]).unwrap();
        assert_eq!(result, RuntimeValue::Bool(false));
    }

    #[test]
    fn is_odd_true() {
        let result = host_integer_is_odd(&[RuntimeValue::Int(7)]).unwrap();
        assert_eq!(result, RuntimeValue::Bool(true));
    }

    #[test]
    fn is_odd_false() {
        let result = host_integer_is_odd(&[RuntimeValue::Int(8)]).unwrap();
        assert_eq!(result, RuntimeValue::Bool(false));
    }

    #[test]
    fn pow_positive() {
        let result = host_integer_pow(&[RuntimeValue::Int(2), RuntimeValue::Int(10)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(1024));
    }

    #[test]
    fn pow_zero_exponent() {
        let result = host_integer_pow(&[RuntimeValue::Int(99), RuntimeValue::Int(0)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(1));
    }

    #[test]
    fn pow_negative_exponent_rejected() {
        let result = host_integer_pow(&[RuntimeValue::Int(2), RuntimeValue::Int(-1)]);
        assert!(result.is_err());
    }

    #[test]
    fn pow_overflow_detected() {
        let result = host_integer_pow(&[RuntimeValue::Int(i64::MAX), RuntimeValue::Int(2)]);
        assert!(result.is_err());
    }

    #[test]
    fn non_int_rejected() {
        let result = host_integer_is_even(&[RuntimeValue::String("4".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn registration_via_call() {
        let registry = HostRegistry::new();
        register_integer_host_functions(&registry);
        let result = registry
            .call(
                "integer_gcd",
                &[RuntimeValue::Int(12), RuntimeValue::Int(8)],
            )
            .unwrap();
        assert_eq!(result, RuntimeValue::Int(4));
    }
}
