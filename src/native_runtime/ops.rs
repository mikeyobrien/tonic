use super::{runtime_value_kind, NativeRuntimeError, NativeRuntimeErrorCode};
use crate::ir::CmpKind;
use crate::runtime::RuntimeValue;

pub(crate) fn add_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match (&left, &right) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => Ok(RuntimeValue::Int(l + r)),
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            Ok(RuntimeValue::Float(format_float(l + r)))
        }
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => {
            let l = *l as f64;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            Ok(RuntimeValue::Float(format_float(l + r)))
        }
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r = *r as f64;
            Ok(RuntimeValue::Float(format_float(l + r)))
        }
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn sub_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match (&left, &right) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => Ok(RuntimeValue::Int(l - r)),
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            Ok(RuntimeValue::Float(format_float(l - r)))
        }
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => {
            let l = *l as f64;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            Ok(RuntimeValue::Float(format_float(l - r)))
        }
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r = *r as f64;
            Ok(RuntimeValue::Float(format_float(l - r)))
        }
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn mul_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match (&left, &right) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => Ok(RuntimeValue::Int(l * r)),
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            Ok(RuntimeValue::Float(format_float(l * r)))
        }
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => {
            let l = *l as f64;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            Ok(RuntimeValue::Float(format_float(l * r)))
        }
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r = *r as f64;
            Ok(RuntimeValue::Float(format_float(l * r)))
        }
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn div_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match (&left, &right) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => {
            if *r == 0 {
                return Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::DivisionByZero,
                    "division by zero",
                    offset,
                ));
            }
            Ok(RuntimeValue::Int(l / r))
        }
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            if r == 0.0 {
                return Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::DivisionByZero,
                    "division by zero",
                    offset,
                ));
            }
            Ok(RuntimeValue::Float(format_float(l / r)))
        }
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => {
            let l = *l as f64;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            if r == 0.0 {
                return Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::DivisionByZero,
                    "division by zero",
                    offset,
                ));
            }
            Ok(RuntimeValue::Float(format_float(l / r)))
        }
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r = *r as f64;
            if r == 0.0 {
                return Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::DivisionByZero,
                    "division by zero",
                    offset,
                ));
            }
            Ok(RuntimeValue::Float(format_float(l / r)))
        }
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn int_div(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;

    if right == 0 {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::DivisionByZero,
            "integer division by zero",
            offset,
        ));
    }

    // Truncating integer division (toward zero)
    Ok(RuntimeValue::Int(left / right))
}

pub(crate) fn rem_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;

    if right == 0 {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::DivisionByZero,
            "remainder by zero",
            offset,
        ));
    }

    Ok(RuntimeValue::Int(left % right))
}

pub(crate) fn cmp_int(
    kind: CmpKind,
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    // Eq, NotEq, StrictEq, and StrictNotEq compare polymorphically without coercion
    if matches!(kind, CmpKind::Eq | CmpKind::NotEq | CmpKind::StrictEq | CmpKind::StrictNotEq) {
        let equal = left == right;
        return Ok(RuntimeValue::Bool(match kind {
            CmpKind::Eq | CmpKind::StrictEq => equal,
            CmpKind::NotEq | CmpKind::StrictNotEq => !equal,
            _ => unreachable!(),
        }));
    }

    let (lf, rf) = match (&left, &right) {
        (RuntimeValue::Int(l), RuntimeValue::Int(r)) => (*l as f64, *r as f64),
        (RuntimeValue::Float(l), RuntimeValue::Float(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            (l, r)
        }
        (RuntimeValue::Int(l), RuntimeValue::Float(r)) => {
            let r: f64 = r.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            (*l as f64, r)
        }
        (RuntimeValue::Float(l), RuntimeValue::Int(r)) => {
            let l: f64 = l.parse().map_err(|_| NativeRuntimeError::badarg(offset))?;
            (l, *r as f64)
        }
        _ => return Err(NativeRuntimeError::badarg(offset)),
    };

    let result = match kind {
        CmpKind::Lt => lf < rf,
        CmpKind::Lte => lf <= rf,
        CmpKind::Gt => lf > rf,
        CmpKind::Gte => lf >= rf,
        CmpKind::Eq | CmpKind::NotEq | CmpKind::StrictEq | CmpKind::StrictNotEq => unreachable!(),
    };

    Ok(RuntimeValue::Bool(result))
}

pub(crate) fn strict_not(
    value: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match value {
        RuntimeValue::Bool(flag) => Ok(RuntimeValue::Bool(!flag)),
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn truthy_bang(value: RuntimeValue) -> RuntimeValue {
    let truthy = !matches!(value, RuntimeValue::Nil | RuntimeValue::Bool(false));
    RuntimeValue::Bool(!truthy)
}

pub(crate) fn concat(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match (left, right) {
        (RuntimeValue::String(l), RuntimeValue::String(r)) => Ok(RuntimeValue::String(l + &r)),
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn in_operator(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let found = match right {
        RuntimeValue::List(items) => items.contains(&left),
        RuntimeValue::Range(start, end) => {
            if let RuntimeValue::Int(value) = left {
                value >= start && value <= end
            } else {
                false
            }
        }
        RuntimeValue::SteppedRange(start, end, step) => {
            if let RuntimeValue::Int(value) = left {
                if step == 0 {
                    false
                } else if step > 0 {
                    value >= start && value <= end && (value - start) % step == 0
                } else {
                    value <= start && value >= end && (start - value) % (-step) == 0
                }
            } else {
                false
            }
        }
        _ => return Err(NativeRuntimeError::badarg(offset)),
    };

    Ok(RuntimeValue::Bool(found))
}

pub(crate) fn list_concat(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match (left, right) {
        (RuntimeValue::List(mut l), RuntimeValue::List(mut r)) => {
            l.append(&mut r);
            Ok(RuntimeValue::List(l))
        }
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn list_subtract(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match (left, right) {
        (RuntimeValue::List(mut l), RuntimeValue::List(r)) => {
            for item in r {
                if let Some(index) = l.iter().position(|x| x == &item) {
                    l.remove(index);
                }
            }
            Ok(RuntimeValue::List(l))
        }
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn range(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Range(left, right))
}

fn expect_int_operand(value: RuntimeValue, offset: usize) -> Result<i64, NativeRuntimeError> {
    match value {
        RuntimeValue::Int(number) => Ok(number),
        RuntimeValue::String(_) => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::BadArg,
            "int operator expects int operands, found string. Hint: String comparison with == is not supported. Use the pin operator in a case expression: `case value do ^expected -> :match; _ -> :no_match end`",
            offset,
        )),
        RuntimeValue::Float(_) => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::BadArg,
            "int operator expects int operands, found float. Hint: For integer operations (+, -, *, div, rem), convert floats to integers first. Use / for float division.",
            offset,
        )),
        other => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::BadArg,
            format!(
                "int operator expects int operands, found {}",
                runtime_value_kind(&other)
            ),
            offset,
        )),
    }
}

pub(crate) fn bitwise_and(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left & right))
}

pub(crate) fn bitwise_or(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left | right))
}

pub(crate) fn bitwise_xor(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left ^ right))
}

pub(crate) fn bitwise_not(
    value: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let value = expect_int_operand(value, offset)?;
    Ok(RuntimeValue::Int(!value))
}

pub(crate) fn bitwise_shift_left(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left << right))
}

pub(crate) fn bitwise_shift_right(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left >> right))
}

pub(crate) fn stepped_range(
    range: RuntimeValue,
    step: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let step = expect_int_operand(step, offset)?;
    match range {
        RuntimeValue::Range(start, end) => Ok(RuntimeValue::SteppedRange(start, end, step)),
        _ => Err(NativeRuntimeError::badarg(offset)),
    }
}

pub(crate) fn kernel_div(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    if right == 0 {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::DivisionByZero,
            "division by zero",
            offset,
        ));
    }
    Ok(RuntimeValue::Int(left / right))
}

pub(crate) fn kernel_rem(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    if right == 0 {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::DivisionByZero,
            "remainder by zero",
            offset,
        ));
    }
    Ok(RuntimeValue::Int(left % right))
}

fn format_float(value: f64) -> String {
    if value.fract() == 0.0 && value.is_finite() {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}
