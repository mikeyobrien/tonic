use super::system::expect_exact_args;
use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn expect_float_or_int(name: &str, args: &[RuntimeValue], index: usize) -> Result<f64, HostError> {
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

fn host_float_to_string(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Float.to_string", args, 1)?;
    match &args[0] {
        RuntimeValue::Float(f) => Ok(RuntimeValue::String(f.clone())),
        RuntimeValue::Int(n) => Ok(RuntimeValue::String(format!("{}.0", n))),
        other => Err(HostError::new(format!(
            "Float.to_string expects numeric argument; found {}",
            super::host_value_kind(other)
        ))),
    }
}

fn host_float_round(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Float.round", args, 2)?;
    let value = expect_float_or_int("Float.round", args, 0)?;
    match &args[1] {
        RuntimeValue::Int(precision) => {
            let factor = 10_f64.powi(*precision as i32);
            let rounded = (value * factor).round() / factor;
            Ok(RuntimeValue::Float(format!("{}", rounded)))
        }
        other => Err(HostError::new(format!(
            "Float.round expects integer precision; found {}",
            super::host_value_kind(other)
        ))),
    }
}

fn host_float_ceil(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Float.ceil", args, 1)?;
    let value = expect_float_or_int("Float.ceil", args, 0)?;
    let result = value.ceil();
    Ok(RuntimeValue::Float(format!("{}", result)))
}

fn host_float_floor(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Float.floor", args, 1)?;
    let value = expect_float_or_int("Float.floor", args, 0)?;
    let result = value.floor();
    Ok(RuntimeValue::Float(format!("{}", result)))
}

pub fn register_float_host_functions(registry: &HostRegistry) {
    registry.register("float_to_string", host_float_to_string);
    registry.register("float_round", host_float_round);
    registry.register("float_ceil", host_float_ceil);
    registry.register("float_floor", host_float_floor);
}
