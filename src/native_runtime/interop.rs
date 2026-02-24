use super::{runtime_value_kind, NativeRuntimeError, NativeRuntimeErrorCode};
use crate::interop::{HostError, HOST_REGISTRY};
use crate::runtime::RuntimeValue;

pub(crate) const TONIC_HOST_INTEROP_ABI_VERSION: u32 = 1;

const PROTOCOL_DISPATCH_TABLE: &[(&str, i64)] = &[("tuple", 1), ("map", 2)];

pub(crate) fn evaluate_protocol_dispatch(
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

pub(crate) fn evaluate_host_call(
    mut args: Vec<RuntimeValue>,
    offset: usize,
) -> Result<RuntimeValue, NativeRuntimeError> {
    let _abi_version = TONIC_HOST_INTEROP_ABI_VERSION;
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
