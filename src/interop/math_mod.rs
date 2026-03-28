use super::system::expect_exact_args;
use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn expect_numeric(name: &str, args: &[RuntimeValue], index: usize) -> Result<f64, HostError> {
    match &args[index] {
        RuntimeValue::Float(f) => Ok(f.parse::<f64>().unwrap_or(0.0)),
        RuntimeValue::Int(n) => Ok(*n as f64),
        other => Err(HostError::new(format!(
            "{} expects numeric argument {}; found {}",
            name,
            index + 1,
            super::host_value_kind(other)
        ))),
    }
}

fn host_math_pow(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.pow", args, 2)?;
    let base = expect_numeric("Math.pow", args, 0)?;
    let exp = expect_numeric("Math.pow", args, 1)?;
    let result = base.powf(exp);
    // Return int if both args are ints and result is a whole number
    if matches!(args[0], RuntimeValue::Int(_))
        && matches!(args[1], RuntimeValue::Int(_))
        && result.fract() == 0.0
        && result.is_finite()
        && result.abs() <= i64::MAX as f64
    {
        Ok(RuntimeValue::Int(result as i64))
    } else {
        Ok(RuntimeValue::Float(format!("{}", result)))
    }
}

fn host_math_sqrt(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.sqrt", args, 1)?;
    let value = expect_numeric("Math.sqrt", args, 0)?;
    if value < 0.0 {
        return Err(HostError::new(
            "Math.sqrt: cannot take square root of negative number",
        ));
    }
    Ok(RuntimeValue::Float(format!("{}", value.sqrt())))
}

fn host_math_abs(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.abs", args, 1)?;
    match &args[0] {
        RuntimeValue::Int(n) => Ok(RuntimeValue::Int(n.abs())),
        RuntimeValue::Float(f) => {
            let v = f.parse::<f64>().unwrap_or(0.0);
            Ok(RuntimeValue::Float(format!("{}", v.abs())))
        }
        other => Err(HostError::new(format!(
            "Math.abs expects numeric argument; found {}",
            super::host_value_kind(other)
        ))),
    }
}

fn host_math_min(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.min", args, 2)?;
    let a = expect_numeric("Math.min", args, 0)?;
    let b = expect_numeric("Math.min", args, 1)?;
    if a <= b {
        Ok(args[0].clone())
    } else {
        Ok(args[1].clone())
    }
}

fn host_math_max(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.max", args, 2)?;
    let a = expect_numeric("Math.max", args, 0)?;
    let b = expect_numeric("Math.max", args, 1)?;
    if a >= b {
        Ok(args[0].clone())
    } else {
        Ok(args[1].clone())
    }
}

fn host_math_log(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.log", args, 1)?;
    let value = expect_numeric("Math.log", args, 0)?;
    if value <= 0.0 {
        return Err(HostError::new("Math.log: argument must be positive"));
    }
    Ok(RuntimeValue::Float(format!("{}", value.ln())))
}

fn host_math_log2(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.log2", args, 1)?;
    let value = expect_numeric("Math.log2", args, 0)?;
    if value <= 0.0 {
        return Err(HostError::new("Math.log2: argument must be positive"));
    }
    Ok(RuntimeValue::Float(format!("{}", value.log2())))
}

fn host_math_log10(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.log10", args, 1)?;
    let value = expect_numeric("Math.log10", args, 0)?;
    if value <= 0.0 {
        return Err(HostError::new("Math.log10: argument must be positive"));
    }
    Ok(RuntimeValue::Float(format!("{}", value.log10())))
}

fn host_math_sin(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.sin", args, 1)?;
    let value = expect_numeric("Math.sin", args, 0)?;
    Ok(RuntimeValue::Float(format!("{}", value.sin())))
}

fn host_math_cos(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.cos", args, 1)?;
    let value = expect_numeric("Math.cos", args, 0)?;
    Ok(RuntimeValue::Float(format!("{}", value.cos())))
}

fn host_math_tan(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.tan", args, 1)?;
    let value = expect_numeric("Math.tan", args, 0)?;
    Ok(RuntimeValue::Float(format!("{}", value.tan())))
}

fn host_math_ceil(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.ceil", args, 1)?;
    let value = expect_numeric("Math.ceil", args, 0)?;
    Ok(RuntimeValue::Int(value.ceil() as i64))
}

fn host_math_floor(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.floor", args, 1)?;
    let value = expect_numeric("Math.floor", args, 0)?;
    Ok(RuntimeValue::Int(value.floor() as i64))
}

fn host_math_round(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Math.round", args, 1)?;
    let value = expect_numeric("Math.round", args, 0)?;
    Ok(RuntimeValue::Int(value.round() as i64))
}

pub fn register_math_host_functions(registry: &HostRegistry) {
    registry.register("math_pow", host_math_pow);
    registry.register("math_sqrt", host_math_sqrt);
    registry.register("math_abs", host_math_abs);
    registry.register("math_min", host_math_min);
    registry.register("math_max", host_math_max);
    registry.register("math_log", host_math_log);
    registry.register("math_log2", host_math_log2);
    registry.register("math_log10", host_math_log10);
    registry.register("math_sin", host_math_sin);
    registry.register("math_cos", host_math_cos);
    registry.register("math_tan", host_math_tan);
    registry.register("math_ceil", host_math_ceil);
    registry.register("math_floor", host_math_floor);
    registry.register("math_round", host_math_round);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn call(name: &str, args: &[RuntimeValue]) -> RuntimeValue {
        HOST_REGISTRY
            .call(name, args)
            .unwrap_or_else(|e| panic!("{name} failed: {e}"))
    }

    fn float(v: f64) -> RuntimeValue {
        RuntimeValue::Float(format!("{}", v))
    }

    fn int(v: i64) -> RuntimeValue {
        RuntimeValue::Int(v)
    }

    fn as_f64(v: &RuntimeValue) -> f64 {
        match v {
            RuntimeValue::Float(f) => f.parse().unwrap(),
            RuntimeValue::Int(n) => *n as f64,
            other => panic!("expected numeric, got {:?}", other),
        }
    }

    #[test]
    fn pow_int_result() {
        assert_eq!(call("math_pow", &[int(2), int(10)]), int(1024));
    }

    #[test]
    fn pow_float_result() {
        let result = as_f64(&call("math_pow", &[float(2.0), float(0.5)]));
        assert!((result - std::f64::consts::SQRT_2).abs() < 1e-10);
    }

    #[test]
    fn sqrt_perfect() {
        let result = as_f64(&call("math_sqrt", &[int(4)]));
        assert!((result - 2.0).abs() < 1e-10);
    }

    #[test]
    fn sqrt_negative_errors() {
        HOST_REGISTRY
            .call("math_sqrt", &[int(-1)])
            .expect_err("sqrt of negative should error");
    }

    #[test]
    fn abs_int() {
        assert_eq!(call("math_abs", &[int(-42)]), int(42));
    }

    #[test]
    fn abs_float() {
        let result = as_f64(&call("math_abs", &[float(-std::f64::consts::PI)]));
        assert!((result - std::f64::consts::PI).abs() < 1e-10);
    }

    #[test]
    fn min_returns_smaller() {
        assert_eq!(call("math_min", &[int(5), int(3)]), int(3));
    }

    #[test]
    fn max_returns_larger() {
        assert_eq!(call("math_max", &[int(5), int(3)]), int(5));
    }

    #[test]
    fn min_mixed_types() {
        // 2 < 3.5, so returns int(2)
        assert_eq!(call("math_min", &[int(2), float(3.5)]), int(2));
    }

    #[test]
    fn log_of_e() {
        let result = as_f64(&call("math_log", &[float(std::f64::consts::E)]));
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn log2_of_8() {
        let result = as_f64(&call("math_log2", &[int(8)]));
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn log10_of_1000() {
        let result = as_f64(&call("math_log10", &[int(1000)]));
        assert!((result - 3.0).abs() < 1e-10);
    }

    #[test]
    fn sin_zero() {
        let result = as_f64(&call("math_sin", &[int(0)]));
        assert!(result.abs() < 1e-10);
    }

    #[test]
    fn cos_zero() {
        let result = as_f64(&call("math_cos", &[int(0)]));
        assert!((result - 1.0).abs() < 1e-10);
    }

    #[test]
    fn tan_zero() {
        let result = as_f64(&call("math_tan", &[int(0)]));
        assert!(result.abs() < 1e-10);
    }

    #[test]
    fn ceil_rounds_up() {
        assert_eq!(call("math_ceil", &[float(2.3)]), int(3));
    }

    #[test]
    fn floor_rounds_down() {
        assert_eq!(call("math_floor", &[float(2.7)]), int(2));
    }

    #[test]
    fn round_nearest() {
        assert_eq!(call("math_round", &[float(2.5)]), int(3));
        assert_eq!(call("math_round", &[float(2.4)]), int(2));
    }

    #[test]
    fn pow_rejects_non_numeric() {
        HOST_REGISTRY
            .call("math_pow", &[RuntimeValue::String("x".into()), int(2)])
            .expect_err("should reject string");
    }

    #[test]
    fn log_rejects_non_positive() {
        HOST_REGISTRY
            .call("math_log", &[int(0)])
            .expect_err("log(0) should error");
        HOST_REGISTRY
            .call("math_log", &[int(-1)])
            .expect_err("log(-1) should error");
    }
}
