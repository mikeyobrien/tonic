use super::{
    clone_tvalue, invoke_runtime_boundary, release_tvalue, retain_tvalue, runtime_to_tvalue,
    tvalue_to_runtime, validate_tvalue, AbiErrorCode, TCallContext, TCallStatus, TValue, TValueTag,
    TONIC_RUNTIME_ABI_VERSION,
};
use crate::runtime::RuntimeValue;

#[test]
fn tvalue_layout_is_stable_for_ffi() {
    assert_eq!(std::mem::size_of::<TValue>(), 16);
    assert_eq!(std::mem::align_of::<TValue>(), 8);
}

#[test]
fn runtime_abi_version_is_v1() {
    assert_eq!(TONIC_RUNTIME_ABI_VERSION, 1);
}

#[test]
fn runtime_roundtrip_supports_collections_and_results() {
    let value = RuntimeValue::ResultOk(Box::new(RuntimeValue::Tuple(
        Box::new(RuntimeValue::Map(vec![
            (RuntimeValue::Atom("name".to_string()), RuntimeValue::Int(7)),
            (
                RuntimeValue::Atom("tags".to_string()),
                RuntimeValue::List(vec![RuntimeValue::String("ok".to_string())]),
            ),
        ])),
        Box::new(RuntimeValue::Keyword(vec![(
            RuntimeValue::Atom("mode".to_string()),
            RuntimeValue::Atom("auto".to_string()),
        )])),
    )));

    let abi = runtime_to_tvalue(value.clone()).expect("roundtrip fixture should encode to abi");
    let decoded = tvalue_to_runtime(abi).expect("roundtrip fixture should decode from abi");

    assert_eq!(decoded, value);
}

#[test]
fn retain_and_release_refcount_are_deterministic_under_stress() {
    let value = runtime_to_tvalue(RuntimeValue::String("hello".to_string()))
        .expect("string should encode as heap value");

    let cloned = clone_tvalue(value).expect("heap clone should retain");
    assert_eq!(cloned, value);

    for _ in 0..256 {
        retain_tvalue(value).expect("retain should succeed while handle is valid");
    }

    for _ in 0..258 {
        release_tvalue(value).expect("release should decrement deterministic refcount");
    }

    let released = release_tvalue(value).expect_err("releasing past zero must fail safely");
    assert_eq!(released.code, AbiErrorCode::InvalidHandle);
}

#[test]
fn invalid_tag_is_reported_instead_of_causing_ub() {
    let malformed = TValue::from_raw_parts(0xFF, 0, 0);

    let error = validate_tvalue(malformed).expect_err("invalid tag should fail validation");

    assert_eq!(error.code, AbiErrorCode::InvalidTag);
}

#[test]
fn tag_handle_mismatch_reports_deterministic_error() {
    let string_value = runtime_to_tvalue(RuntimeValue::String("hello".to_string()))
        .expect("string should encode as heap value");

    let malformed = TValue::from_raw_parts(
        TValueTag::Map as u8,
        string_value.ownership,
        string_value.payload,
    );

    let error = tvalue_to_runtime(malformed).expect_err("tag mismatch must fail safely");
    assert_eq!(error.code, AbiErrorCode::TagHandleMismatch);

    release_tvalue(string_value).expect("cleanup release should succeed");
}

#[test]
fn runtime_boundary_rejects_wrong_abi_version() {
    let args = [runtime_to_tvalue(RuntimeValue::Int(1)).expect("int should encode")];
    let mut ctx = TCallContext::from_slice(&args);
    ctx.abi_version = TONIC_RUNTIME_ABI_VERSION + 1;

    let result = invoke_runtime_boundary(&ctx, |_args| Ok(RuntimeValue::Int(1)));

    assert_eq!(result.status, TCallStatus::InvalidAbi);
}

#[test]
fn runtime_boundary_catches_panics_and_returns_error_status() {
    let ctx = TCallContext::from_slice(&[]);

    let result = invoke_runtime_boundary(&ctx, |_args| panic!("boom"));

    assert_eq!(result.status, TCallStatus::Panic);
}
