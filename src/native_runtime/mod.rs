pub(crate) mod boundary;
pub(crate) mod collections;
pub(crate) mod ops;
pub(crate) mod pattern;
#[cfg(test)]
mod tests;

use crate::interop::{HostError, HOST_REGISTRY};
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
            evaluate_protocol_dispatch(value, offset)
        }
        "host_call" => evaluate_host_call(args, offset),
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
        RuntimeValue::Range(_, _) => "range",
        RuntimeValue::Closure(_) => "function",
    }
}

const PROTOCOL_DISPATCH_TABLE: &[(&str, i64)] = &[("tuple", 1), ("map", 2)];

fn evaluate_protocol_dispatch(
    value: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let implementation = PROTOCOL_DISPATCH_TABLE
        .iter()
        .find_map(|(kind, implementation)| {
            (runtime_value_kind(&value) == *kind).then_some(*implementation)
        })
        .ok_or_else(|| {
            NativeRuntimeError::at_offset(
                NativeRuntimeErrorCode::BadArg,
                format!(
                    "protocol_dispatch has no implementation for {}",
                    runtime_value_kind(&value)
                ),
                offset,
            )
        })?;

    Ok(RuntimeValue::Int(implementation))
}

fn evaluate_host_call(
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    if args.is_empty() {
        return Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::ArityMismatch,
            "host_call requires at least 1 argument (host function key)",
            offset,
        ));
    }

    let key = args.remove(0);
    let key_str = match key {
        RuntimeValue::Atom(s) => s,
        other => {
            return Err(NativeRuntimeError::at_offset(
                NativeRuntimeErrorCode::BadArg,
                format!(
                    "host_call first argument must be an atom (host key), found {}",
                    runtime_value_kind(&other)
                ),
                offset,
            ));
        }
    };

    HOST_REGISTRY.call(&key_str, &args).map_err(|e: HostError| {
        NativeRuntimeError::at_offset(NativeRuntimeErrorCode::BadArg, e.to_string(), offset)
    })
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
