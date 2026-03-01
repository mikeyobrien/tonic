use super::{runtime_value_kind, NativeRuntimeError, NativeRuntimeErrorCode};
use crate::ir::CmpKind;
use crate::runtime::RuntimeValue;

pub(crate) fn add_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left + right))
}

pub(crate) fn sub_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left - right))
}

pub(crate) fn mul_int(
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;
    Ok(RuntimeValue::Int(left * right))
}

pub(crate) fn div_int(
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

pub(crate) fn cmp_int(
    kind: CmpKind,
    left: RuntimeValue,
    right: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    // StrictEq and StrictNotEq compare without coercion
    if kind == CmpKind::StrictEq || kind == CmpKind::StrictNotEq {
        let equal = left == right;
        return Ok(RuntimeValue::Bool(if kind == CmpKind::StrictEq { equal } else { !equal }));
    }

    let left = expect_int_operand(left, offset)?;
    let right = expect_int_operand(right, offset)?;

    let result = match kind {
        CmpKind::Eq => left == right,
        CmpKind::NotEq => left != right,
        CmpKind::Lt => left < right,
        CmpKind::Lte => left <= right,
        CmpKind::Gt => left > right,
        CmpKind::Gte => left >= right,
        CmpKind::StrictEq | CmpKind::StrictNotEq => unreachable!(),
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
