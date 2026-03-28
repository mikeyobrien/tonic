use super::system::expect_exact_args;
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn require_int(name: &str, val: &RuntimeValue) -> Result<i64, HostError> {
    match val {
        RuntimeValue::Int(n) => Ok(*n),
        other => Err(HostError::new(format!(
            "{} expects integer arguments, found {}",
            name,
            host_value_kind(other)
        ))),
    }
}

fn host_bitwise_band(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Bitwise.band", args, 2)?;
    let a = require_int("Bitwise.band", &args[0])?;
    let b = require_int("Bitwise.band", &args[1])?;
    Ok(RuntimeValue::Int(a & b))
}

fn host_bitwise_bor(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Bitwise.bor", args, 2)?;
    let a = require_int("Bitwise.bor", &args[0])?;
    let b = require_int("Bitwise.bor", &args[1])?;
    Ok(RuntimeValue::Int(a | b))
}

fn host_bitwise_bxor(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Bitwise.bxor", args, 2)?;
    let a = require_int("Bitwise.bxor", &args[0])?;
    let b = require_int("Bitwise.bxor", &args[1])?;
    Ok(RuntimeValue::Int(a ^ b))
}

fn host_bitwise_bnot(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Bitwise.bnot", args, 1)?;
    let a = require_int("Bitwise.bnot", &args[0])?;
    Ok(RuntimeValue::Int(!a))
}

fn host_bitwise_bsl(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Bitwise.bsl", args, 2)?;
    let a = require_int("Bitwise.bsl", &args[0])?;
    let shift = require_int("Bitwise.bsl", &args[1])?;
    if !(0..=63).contains(&shift) {
        return Err(HostError::new(format!(
            "Bitwise.bsl: shift amount must be 0..63, got {}",
            shift
        )));
    }
    Ok(RuntimeValue::Int(a << shift))
}

fn host_bitwise_bsr(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Bitwise.bsr", args, 2)?;
    let a = require_int("Bitwise.bsr", &args[0])?;
    let shift = require_int("Bitwise.bsr", &args[1])?;
    if !(0..=63).contains(&shift) {
        return Err(HostError::new(format!(
            "Bitwise.bsr: shift amount must be 0..63, got {}",
            shift
        )));
    }
    Ok(RuntimeValue::Int(a >> shift))
}

pub fn register_bitwise_host_functions(registry: &HostRegistry) {
    registry.register("bitwise_band", host_bitwise_band);
    registry.register("bitwise_bor", host_bitwise_bor);
    registry.register("bitwise_bxor", host_bitwise_bxor);
    registry.register("bitwise_bnot", host_bitwise_bnot);
    registry.register("bitwise_bsl", host_bitwise_bsl);
    registry.register("bitwise_bsr", host_bitwise_bsr);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_masks_bits() {
        let result =
            host_bitwise_band(&[RuntimeValue::Int(0b1100), RuntimeValue::Int(0b1010)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(0b1000));
    }

    #[test]
    fn bor_combines_bits() {
        let result =
            host_bitwise_bor(&[RuntimeValue::Int(0b1100), RuntimeValue::Int(0b1010)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(0b1110));
    }

    #[test]
    fn bxor_toggles_bits() {
        let result =
            host_bitwise_bxor(&[RuntimeValue::Int(0b1100), RuntimeValue::Int(0b1010)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(0b0110));
    }

    #[test]
    fn bnot_complements() {
        let result = host_bitwise_bnot(&[RuntimeValue::Int(0)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(-1)); // !0 == -1 in two's complement
    }

    #[test]
    fn bsl_shifts_left() {
        let result = host_bitwise_bsl(&[RuntimeValue::Int(1), RuntimeValue::Int(4)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(16));
    }

    #[test]
    fn bsr_shifts_right() {
        let result = host_bitwise_bsr(&[RuntimeValue::Int(16), RuntimeValue::Int(4)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(1));
    }

    #[test]
    fn bsl_rejects_negative_shift() {
        let result = host_bitwise_bsl(&[RuntimeValue::Int(1), RuntimeValue::Int(-1)]);
        assert!(result.is_err());
    }

    #[test]
    fn bsr_rejects_shift_over_63() {
        let result = host_bitwise_bsr(&[RuntimeValue::Int(1), RuntimeValue::Int(64)]);
        assert!(result.is_err());
    }

    #[test]
    fn band_rejects_non_integer() {
        let result =
            host_bitwise_band(&[RuntimeValue::Int(1), RuntimeValue::Float("1.0".to_string())]);
        assert!(result.is_err());
    }

    #[test]
    fn band_rejects_wrong_arity() {
        let result = host_bitwise_band(&[RuntimeValue::Int(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn bnot_rejects_wrong_arity() {
        let result = host_bitwise_bnot(&[RuntimeValue::Int(1), RuntimeValue::Int(2)]);
        assert!(result.is_err());
    }

    #[test]
    fn negative_operands_work() {
        let result = host_bitwise_band(&[RuntimeValue::Int(-1), RuntimeValue::Int(0xFF)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(0xFF));
    }

    #[test]
    fn registration_via_call() {
        let registry = HostRegistry::new();
        register_bitwise_host_functions(&registry);
        let result = registry
            .call(
                "bitwise_band",
                &[RuntimeValue::Int(3), RuntimeValue::Int(5)],
            )
            .unwrap();
        assert_eq!(result, RuntimeValue::Int(1));
    }
}
