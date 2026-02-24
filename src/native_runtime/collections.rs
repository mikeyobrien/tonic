use super::{runtime_value_kind, NativeRuntimeError, NativeRuntimeErrorCode};
use crate::runtime::RuntimeValue;

pub(crate) fn tuple(left: RuntimeValue, right: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Tuple(Box::new(left), Box::new(right))
}

pub(crate) fn list(items: Vec<RuntimeValue>) -> RuntimeValue {
    RuntimeValue::List(items)
}

pub(crate) fn map_empty() -> RuntimeValue {
    RuntimeValue::Map(Vec::new())
}

pub(crate) fn map(key: RuntimeValue, value: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Map(vec![(key, value)])
}

pub(crate) fn map_put(
    base: RuntimeValue,
    key: RuntimeValue,
    value: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match base {
        RuntimeValue::Map(mut entries) => {
            if let Some(existing) = entries
                .iter_mut()
                .find(|(entry_key, _entry_value)| *entry_key == key)
            {
                existing.1 = value;
            } else {
                entries.push((key, value));
            }

            Ok(RuntimeValue::Map(entries))
        }
        _ => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::BadArg,
            format!(
                "expected map base for put, found {}",
                runtime_value_kind(&base)
            ),
            offset,
        )),
    }
}

pub(crate) fn map_update(
    base: RuntimeValue,
    key: RuntimeValue,
    value: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match base {
        RuntimeValue::Map(mut entries) => {
            if let Some(existing) = entries
                .iter_mut()
                .find(|(entry_key, _entry_value)| *entry_key == key)
            {
                existing.1 = value;
                return Ok(RuntimeValue::Map(entries));
            }

            Err(NativeRuntimeError::at_offset(
                NativeRuntimeErrorCode::BadArg,
                format!("key {} not found in map", key.render()),
                offset,
            ))
        }
        _ => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::BadArg,
            format!(
                "expected map base for update, found {}",
                runtime_value_kind(&base)
            ),
            offset,
        )),
    }
}

pub(crate) fn map_access(
    base: RuntimeValue,
    key: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match base {
        RuntimeValue::Map(entries) => Ok(entries
            .into_iter()
            .find_map(|(entry_key, value)| (entry_key == key).then_some(value))
            .unwrap_or(RuntimeValue::Nil)),
        _ => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::BadArg,
            format!(
                "expected map base for access, found {}",
                runtime_value_kind(&base)
            ),
            offset,
        )),
    }
}

pub(crate) fn keyword(key: RuntimeValue, value: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Keyword(vec![(key, value)])
}

pub(crate) fn keyword_append(
    base: RuntimeValue,
    key: RuntimeValue,
    value: RuntimeValue,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    match base {
        RuntimeValue::Keyword(mut entries) => {
            entries.push((key, value));
            Ok(RuntimeValue::Keyword(entries))
        }
        _ => Err(NativeRuntimeError::at_offset(
            NativeRuntimeErrorCode::BadArg,
            format!(
                "expected keyword base for append, found {}",
                runtime_value_kind(&base)
            ),
            offset,
        )),
    }
}
