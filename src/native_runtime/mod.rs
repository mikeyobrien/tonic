pub(crate) mod boundary;
pub(crate) mod collections;
pub(crate) mod interop;
pub(crate) mod ops;
pub(crate) mod pattern;
#[cfg(test)]
mod tests;

use crate::guard_builtins;
use crate::runtime::RuntimeValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeRuntimeErrorCode {
    ArityMismatch,
    BadArg,
    DivisionByZero,
    UnsupportedBuiltin,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeRuntimeError {
    code: NativeRuntimeErrorCode,
    message: String,
    offset: usize,
}

impl NativeRuntimeError {
    pub(crate) fn at_offset(
        code: NativeRuntimeErrorCode,
        message: impl Into<String>,
        offset: usize,
    ) -> Self {
        Self {
            code,
            message: message.into(),
            offset,
        }
    }

    pub(crate) fn badarg(offset: usize) -> Self {
        Self::at_offset(NativeRuntimeErrorCode::BadArg, "badarg", offset)
    }

    pub(crate) fn code(&self) -> NativeRuntimeErrorCode {
        self.code
    }

    pub(crate) fn message(&self) -> &str {
        &self.message
    }

    pub(crate) fn offset(&self) -> usize {
        self.offset
    }
}

impl std::fmt::Display for NativeRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at offset {}", self.message, self.offset)
    }
}

impl std::error::Error for NativeRuntimeError {}

pub(crate) fn evaluate_builtin_call(
    name: &str,
    args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    if guard_builtins::is_guard_builtin(name) {
        let value = expect_single_builtin_arg(name, args, offset)?;
        let result = guard_builtins::evaluate_guard_builtin(name, &value).ok_or_else(|| {
            NativeRuntimeError::at_offset(
                NativeRuntimeErrorCode::UnsupportedBuiltin,
                format!("unsupported builtin call in runtime evaluator: {name}"),
                offset,
            )
        })?;

        return Ok(RuntimeValue::Bool(result));
    }

    match name {
        "ok" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            Ok(RuntimeValue::ResultOk(Box::new(arg)))
        }
        "err" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            Ok(RuntimeValue::ResultErr(Box::new(arg)))
        }
        "tuple" => {
            let (left, right) = expect_pair_builtin_args(name, args, offset)?;
            Ok(collections::tuple(left, right))
        }
        "list" => Ok(collections::list(args)),
        "map_empty" => {
            if !args.is_empty() {
                return Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::ArityMismatch,
                    format!(
                        "arity mismatch for runtime builtin map_empty: expected 0 args, found {}",
                        args.len()
                    ),
                    offset,
                ));
            }

            Ok(collections::map_empty())
        }
        "map" => {
            let (key, value) = expect_pair_builtin_args(name, args, offset)?;
            Ok(collections::map(key, value))
        }
        "map_put" => {
            let (base, key, value) = expect_triple_builtin_args(name, args, offset)?;
            collections::map_put(base, key, value, offset)
        }
        "map_update" => {
            let (base, key, value) = expect_triple_builtin_args(name, args, offset)?;
            collections::map_update(base, key, value, offset)
        }
        "map_access" => {
            let (base, key) = expect_pair_builtin_args(name, args, offset)?;
            collections::map_access(base, key, offset)
        }
        "keyword" => {
            let (key, value) = expect_pair_builtin_args(name, args, offset)?;
            Ok(collections::keyword(key, value))
        }
        "keyword_append" => {
            let (base, key, value) = expect_triple_builtin_args(name, args, offset)?;
            collections::keyword_append(base, key, value, offset)
        }
        "protocol_dispatch" => {
            let value = expect_single_builtin_arg(name, args, offset)?;
            interop::evaluate_protocol_dispatch(value, offset)
        }
        "host_call" => interop::evaluate_host_call(args, offset),
        "div" => {
            let (left, right) = expect_pair_builtin_args(name, args, offset)?;
            ops::kernel_div(left, right, offset)
        }
        "rem" => {
            let (left, right) = expect_pair_builtin_args(name, args, offset)?;
            ops::kernel_rem(left, right, offset)
        }
        "byte_size" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::Binary(ref items) => Ok(RuntimeValue::Int(items.len() as i64)),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::UnsupportedBuiltin,
                    format!(
                        "byte_size expects a binary, found {}",
                        runtime_value_kind(&arg)
                    ),
                    offset,
                )),
            }
        }
        "bit_size" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::Binary(ref items) => Ok(RuntimeValue::Int((items.len() * 8) as i64)),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::UnsupportedBuiltin,
                    format!(
                        "bit_size expects a binary, found {}",
                        runtime_value_kind(&arg)
                    ),
                    offset,
                )),
            }
        }
        "abs" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::Int(n) => Ok(RuntimeValue::Int(n.abs())),
                RuntimeValue::Float(ref s) => {
                    let f: f64 = s.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Float(format!("{}", f.abs())))
                }
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("abs expects a number, found {}", runtime_value_kind(&arg)),
                    offset,
                )),
            }
        }
        "length" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::List(ref items) => Ok(RuntimeValue::Int(items.len() as i64)),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("length expects a list, found {}", runtime_value_kind(&arg)),
                    offset,
                )),
            }
        }
        "hd" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::List(ref items) if !items.is_empty() => Ok(items[0].clone()),
                RuntimeValue::List(_) => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    "hd called on empty list",
                    offset,
                )),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("hd expects a list, found {}", runtime_value_kind(&arg)),
                    offset,
                )),
            }
        }
        "tl" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::List(ref items) if !items.is_empty() => {
                    Ok(RuntimeValue::List(items[1..].to_vec()))
                }
                RuntimeValue::List(_) => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    "tl called on empty list",
                    offset,
                )),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("tl expects a list, found {}", runtime_value_kind(&arg)),
                    offset,
                )),
            }
        }
        "elem" => {
            let (tuple, index) = expect_pair_builtin_args(name, args, offset)?;
            match (&tuple, &index) {
                (RuntimeValue::Tuple(left, right), RuntimeValue::Int(i)) => match *i {
                    0 => Ok(*left.clone()),
                    1 => Ok(*right.clone()),
                    _ => Err(NativeRuntimeError::at_offset(
                        NativeRuntimeErrorCode::BadArg,
                        format!("elem index {} out of range for 2-element tuple", i),
                        offset,
                    )),
                },
                (RuntimeValue::Tuple(_, _), _) => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!(
                        "elem index must be an integer, found {}",
                        runtime_value_kind(&index)
                    ),
                    offset,
                )),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("elem expects a tuple, found {}", runtime_value_kind(&tuple)),
                    offset,
                )),
            }
        }
        "tuple_size" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::Tuple(_, _) => Ok(RuntimeValue::Int(2)),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!(
                        "tuple_size expects a tuple, found {}",
                        runtime_value_kind(&arg)
                    ),
                    offset,
                )),
            }
        }
        "to_string" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            let str_value = match arg {
                RuntimeValue::String(s) => s,
                RuntimeValue::Int(i) => i.to_string(),
                RuntimeValue::Float(f) => f.clone(),
                RuntimeValue::Bool(b) => b.to_string(),
                RuntimeValue::Nil => String::new(),
                RuntimeValue::Atom(a) => a,
                other => other.render(),
            };
            Ok(RuntimeValue::String(str_value))
        }
        "max" => {
            let (a, b) = expect_pair_builtin_args(name, args, offset)?;
            match (&a, &b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => Ok(RuntimeValue::Int(*x.max(y))),
                (RuntimeValue::Float(x), RuntimeValue::Float(y)) => {
                    let fx: f64 = x.parse().unwrap_or(0.0);
                    let fy: f64 = y.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Float(format!("{}", fx.max(fy))))
                }
                (RuntimeValue::Int(x), RuntimeValue::Float(y)) => {
                    let fy: f64 = y.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Float(format!("{}", (*x as f64).max(fy))))
                }
                (RuntimeValue::Float(x), RuntimeValue::Int(y)) => {
                    let fx: f64 = x.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Float(format!("{}", fx.max(*y as f64))))
                }
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!(
                        "max expects two numbers, found {} and {}",
                        runtime_value_kind(&a),
                        runtime_value_kind(&b)
                    ),
                    offset,
                )),
            }
        }
        "min" => {
            let (a, b) = expect_pair_builtin_args(name, args, offset)?;
            match (&a, &b) {
                (RuntimeValue::Int(x), RuntimeValue::Int(y)) => Ok(RuntimeValue::Int(*x.min(y))),
                (RuntimeValue::Float(x), RuntimeValue::Float(y)) => {
                    let fx: f64 = x.parse().unwrap_or(0.0);
                    let fy: f64 = y.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Float(format!("{}", fx.min(fy))))
                }
                (RuntimeValue::Int(x), RuntimeValue::Float(y)) => {
                    let fy: f64 = y.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Float(format!("{}", (*x as f64).min(fy))))
                }
                (RuntimeValue::Float(x), RuntimeValue::Int(y)) => {
                    let fx: f64 = x.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Float(format!("{}", fx.min(*y as f64))))
                }
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!(
                        "min expects two numbers, found {} and {}",
                        runtime_value_kind(&a),
                        runtime_value_kind(&b)
                    ),
                    offset,
                )),
            }
        }
        "round" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::Int(n) => Ok(RuntimeValue::Int(n)),
                RuntimeValue::Float(ref s) => {
                    let f: f64 = s.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Int(f.round() as i64))
                }
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("round expects a number, found {}", runtime_value_kind(&arg)),
                    offset,
                )),
            }
        }
        "trunc" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::Int(n) => Ok(RuntimeValue::Int(n)),
                RuntimeValue::Float(ref s) => {
                    let f: f64 = s.parse().unwrap_or(0.0);
                    Ok(RuntimeValue::Int(f.trunc() as i64))
                }
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("trunc expects a number, found {}", runtime_value_kind(&arg)),
                    offset,
                )),
            }
        }
        "map_size" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            match arg {
                RuntimeValue::Map(ref entries) => Ok(RuntimeValue::Int(entries.len() as i64)),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!("map_size expects a map, found {}", runtime_value_kind(&arg)),
                    offset,
                )),
            }
        }
        "put_elem" => {
            if args.len() != 3 {
                return Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::ArityMismatch,
                    format!(
                        "arity mismatch for put_elem: expected 3 args, found {}",
                        args.len()
                    ),
                    offset,
                ));
            }
            let tuple = args[0].clone();
            let index = args[1].clone();
            let value = args[2].clone();
            match (&tuple, &index) {
                (RuntimeValue::Tuple(left, right), RuntimeValue::Int(i)) => match *i {
                    0 => Ok(RuntimeValue::Tuple(Box::new(value), right.clone())),
                    1 => Ok(RuntimeValue::Tuple(left.clone(), Box::new(value))),
                    _ => Err(NativeRuntimeError::at_offset(
                        NativeRuntimeErrorCode::BadArg,
                        format!("put_elem index {} out of range for 2-element tuple", i),
                        offset,
                    )),
                },
                (RuntimeValue::Tuple(_, _), _) => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!(
                        "put_elem index must be an integer, found {}",
                        runtime_value_kind(&index)
                    ),
                    offset,
                )),
                _ => Err(NativeRuntimeError::at_offset(
                    NativeRuntimeErrorCode::BadArg,
                    format!(
                        "put_elem expects a tuple, found {}",
                        runtime_value_kind(&tuple)
                    ),
                    offset,
                )),
            }
        }
        "inspect" => {
            let arg = expect_single_builtin_arg(name, args, offset)?;
            Ok(RuntimeValue::String(arg.render()))
        }
        _ => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::UnsupportedBuiltin,
            format!("unsupported builtin call in runtime evaluator: {name}"),
            offset,
        )),
    }
}

pub(crate) fn runtime_value_kind(value: &RuntimeValue) -> &'static str {
    match value {
        RuntimeValue::Int(_) => "int",
        RuntimeValue::Float(_) => "float",
        RuntimeValue::Bool(_) => "bool",
        RuntimeValue::Nil => "nil",
        RuntimeValue::String(_) => "string",
        RuntimeValue::Atom(_) => "atom",
        RuntimeValue::ResultOk(_) | RuntimeValue::ResultErr(_) => "result",
        RuntimeValue::Tuple(_, _) => "tuple",
        RuntimeValue::Map(_) => "map",
        RuntimeValue::Keyword(_) => "keyword",
        RuntimeValue::List(_) => "list",
        RuntimeValue::Binary(_) => "binary",
        RuntimeValue::Range(_, _) => "range",
        RuntimeValue::SteppedRange(_, _, _) => "stepped_range",
        RuntimeValue::Closure(_) => "function",
    }
}

fn expect_single_builtin_arg(
    name: &str,
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    if args.len() != 1 {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::ArityMismatch,
            format!(
                "arity mismatch for runtime builtin {name}: expected 1 args, found {}",
                args.len()
            ),
            offset,
        ));
    }

    Ok(args
        .pop()
        .expect("arity check should guarantee one builtin argument"))
}

fn expect_pair_builtin_args(
    name: &str,
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<(RuntimeValue, RuntimeValue), NativeRuntimeError> {
    if args.len() != 2 {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::ArityMismatch,
            format!(
                "arity mismatch for runtime builtin {name}: expected 2 args, found {}",
                args.len()
            ),
            offset,
        ));
    }

    let right = args
        .pop()
        .expect("arity check should guarantee second builtin argument");
    let left = args
        .pop()
        .expect("arity check should guarantee first builtin argument");

    Ok((left, right))
}

fn expect_triple_builtin_args(
    name: &str,
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<(RuntimeValue, RuntimeValue, RuntimeValue), NativeRuntimeError> {
    if args.len() != 3 {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::ArityMismatch,
            format!(
                "arity mismatch for runtime builtin {name}: expected 3 args, found {}",
                args.len()
            ),
            offset,
        ));
    }

    let third = args
        .pop()
        .expect("arity check should guarantee third builtin argument");
    let second = args
        .pop()
        .expect("arity check should guarantee second builtin argument");
    let first = args
        .pop()
        .expect("arity check should guarantee first builtin argument");

    Ok((first, second, third))
}
