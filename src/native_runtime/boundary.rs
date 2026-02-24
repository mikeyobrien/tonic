use super::{collections, interop, ops, NativeRuntimeError, NativeRuntimeErrorCode};
use crate::ir::CmpKind;
use crate::native_abi::{
    invoke_runtime_boundary, AbiError, AbiErrorCode, TCallContext, TCallResult,
};
use crate::runtime::RuntimeValue;

#[no_mangle]
pub extern "C" fn tonic_rt_add_int(ctx: TCallContext) -> TCallResult {
    invoke_runtime_boundary(&ctx, |args| {
        let (left, right) = expect_pair_args("tonic_rt_add_int", args)?;
        ops::add_int(left, right, 0).map_err(map_error)
    })
}

#[no_mangle]
pub extern "C" fn tonic_rt_cmp_int_eq(ctx: TCallContext) -> TCallResult {
    invoke_runtime_boundary(&ctx, |args| {
        let (left, right) = expect_pair_args("tonic_rt_cmp_int_eq", args)?;
        ops::cmp_int(CmpKind::Eq, left, right, 0).map_err(map_error)
    })
}

#[no_mangle]
pub extern "C" fn tonic_rt_map_put(ctx: TCallContext) -> TCallResult {
    invoke_runtime_boundary(&ctx, |args| {
        let (base, key, value) = expect_triple_args("tonic_rt_map_put", args)?;
        collections::map_put(base, key, value, 0).map_err(map_error)
    })
}

#[no_mangle]
pub extern "C" fn tonic_rt_host_call(ctx: TCallContext) -> TCallResult {
    invoke_runtime_boundary(&ctx, |args| {
        interop::evaluate_host_call(args.to_vec(), 0).map_err(map_error)
    })
}

#[no_mangle]
pub extern "C" fn tonic_rt_protocol_dispatch(ctx: TCallContext) -> TCallResult {
    invoke_runtime_boundary(&ctx, |args| {
        let value = expect_single_arg("tonic_rt_protocol_dispatch", args)?;
        interop::evaluate_protocol_dispatch(value, 0).map_err(map_error)
    })
}

fn expect_single_arg(helper: &str, args: &[RuntimeValue]) -> Result<RuntimeValue, AbiError> {
    if args.len() != 1 {
        return Err(AbiError::new(
            AbiErrorCode::InvalidCallFrame,
            format!(
                "arity mismatch for native runtime helper {helper}: expected 1 args, found {}",
                args.len()
            ),
        ));
    }

    Ok(args[0].clone())
}

fn expect_pair_args(
    helper: &str,
    args: &[RuntimeValue],
) -> Result<(RuntimeValue, RuntimeValue), AbiError> {
    if args.len() != 2 {
        return Err(AbiError::new(
            AbiErrorCode::InvalidCallFrame,
            format!(
                "arity mismatch for native runtime helper {helper}: expected 2 args, found {}",
                args.len()
            ),
        ));
    }

    Ok((args[0].clone(), args[1].clone()))
}

fn expect_triple_args(
    helper: &str,
    args: &[RuntimeValue],
) -> Result<(RuntimeValue, RuntimeValue, RuntimeValue), AbiError> {
    if args.len() != 3 {
        return Err(AbiError::new(
            AbiErrorCode::InvalidCallFrame,
            format!(
                "arity mismatch for native runtime helper {helper}: expected 3 args, found {}",
                args.len()
            ),
        ));
    }

    Ok((args[0].clone(), args[1].clone(), args[2].clone()))
}

fn map_error(error: NativeRuntimeError) -> AbiError {
    let code = match error.code() {
        NativeRuntimeErrorCode::ArityMismatch => AbiErrorCode::InvalidCallFrame,
        NativeRuntimeErrorCode::BadArg => AbiErrorCode::InvalidCallFrame,
        NativeRuntimeErrorCode::DivisionByZero => AbiErrorCode::InvalidCallFrame,
        NativeRuntimeErrorCode::UnsupportedBuiltin => AbiErrorCode::InvalidCallFrame,
    };

    AbiError::new(code, error.to_string())
}
