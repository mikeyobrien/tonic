use super::{
    boundary::{
        tonic_rt_add_int, tonic_rt_cmp_int_eq, tonic_rt_host_call, tonic_rt_map_put,
        tonic_rt_protocol_dispatch,
    },
    collections, evaluate_builtin_call, interop, ops, pattern,
};
use crate::ir::{CmpKind, IrMapPatternEntry, IrPattern};
use crate::native_abi::{runtime_to_tvalue, tvalue_to_runtime, TCallContext, TCallStatus};
use crate::runtime::RuntimeValue;
use std::collections::HashMap;

#[test]
fn host_interop_abi_version_is_v1() {
    assert_eq!(interop::TONIC_HOST_INTEROP_ABI_VERSION, 1);
}

#[test]
fn ops_cover_numeric_logical_and_comparisons() {
    assert_eq!(
        ops::add_int(RuntimeValue::Int(2), RuntimeValue::Int(3), 11)
            .expect("add helper should support ints"),
        RuntimeValue::Int(5)
    );

    assert_eq!(
        ops::div_int(RuntimeValue::Int(10), RuntimeValue::Int(2), 12)
            .expect("div helper should support ints"),
        RuntimeValue::Int(5)
    );

    assert_eq!(
        ops::cmp_int(CmpKind::Gte, RuntimeValue::Int(7), RuntimeValue::Int(7), 13)
            .expect("cmp helper should support ints"),
        RuntimeValue::Bool(true)
    );

    assert_eq!(
        ops::strict_not(RuntimeValue::Bool(false), 14).expect("strict not should support booleans"),
        RuntimeValue::Bool(true)
    );

    assert_eq!(
        ops::truthy_bang(RuntimeValue::Nil),
        RuntimeValue::Bool(true)
    );
    assert_eq!(
        ops::truthy_bang(RuntimeValue::Int(1)),
        RuntimeValue::Bool(false)
    );
}

#[test]
fn collections_cover_constructors_and_mutations() {
    let tuple = collections::tuple(RuntimeValue::Int(1), RuntimeValue::Int(2));
    assert_eq!(tuple.render(), "{1, 2}");

    let list = collections::list(vec![RuntimeValue::Int(1), RuntimeValue::Int(2)]);
    assert_eq!(list.render(), "[1, 2]");

    let map = collections::map(RuntimeValue::Atom("name".to_string()), RuntimeValue::Int(1));
    let map = collections::map_put(
        map,
        RuntimeValue::Atom("name".to_string()),
        RuntimeValue::Int(2),
        27,
    )
    .expect("map_put should update existing key");

    let map = collections::map_put(
        map,
        RuntimeValue::Atom("age".to_string()),
        RuntimeValue::Int(3),
        28,
    )
    .expect("map_put should append missing key");

    let age = collections::map_access(map.clone(), RuntimeValue::Atom("age".to_string()), 29)
        .expect("map_access should return existing key");
    assert_eq!(age, RuntimeValue::Int(3));

    let updated = collections::map_update(
        map,
        RuntimeValue::Atom("age".to_string()),
        RuntimeValue::Int(4),
        30,
    )
    .expect("map_update should update existing key");
    assert_eq!(updated.render(), "%{:name => 2, :age => 4}");

    let keyword =
        collections::keyword(RuntimeValue::Atom("mode".to_string()), RuntimeValue::Int(1));
    let keyword = collections::keyword_append(
        keyword,
        RuntimeValue::Atom("mode".to_string()),
        RuntimeValue::Int(2),
        31,
    )
    .expect("keyword_append should append entries");
    assert_eq!(keyword.render(), "[mode: 1, mode: 2]");
}

#[test]
fn pattern_helpers_cover_case_pattern_checks() {
    let subject = RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("ok".to_string())),
        Box::new(RuntimeValue::Int(7)),
    );

    let pattern = IrPattern::Tuple {
        items: vec![
            IrPattern::Atom {
                value: "ok".to_string(),
            },
            IrPattern::Bind {
                name: "value".to_string(),
            },
        ],
    };

    let mut bindings = HashMap::new();
    let matched = pattern::match_pattern(&subject, &pattern, &HashMap::new(), &mut bindings);
    assert!(matched);
    assert_eq!(bindings.get("value"), Some(&RuntimeValue::Int(7)));

    let branches = vec![
        IrPattern::Atom {
            value: "error".to_string(),
        },
        pattern,
    ];

    let selected = pattern::select_case_branch(&subject, &branches, &HashMap::new())
        .expect("case selection should choose tuple branch");
    assert_eq!(selected.0, 1);
    assert_eq!(selected.1.get("value"), Some(&RuntimeValue::Int(7)));

    let map_subject = RuntimeValue::Map(vec![(
        RuntimeValue::Atom("name".to_string()),
        RuntimeValue::String("tonic".to_string()),
    )]);
    let map_pattern = IrPattern::Map {
        entries: vec![IrMapPatternEntry {
            key: IrPattern::Atom {
                value: "name".to_string(),
            },
            value: IrPattern::Bind {
                name: "label".to_string(),
            },
        }],
    };

    let mut map_bindings = HashMap::new();
    assert!(pattern::match_pattern(
        &map_subject,
        &map_pattern,
        &HashMap::new(),
        &mut map_bindings
    ));
    assert_eq!(
        map_bindings.get("label"),
        Some(&RuntimeValue::String("tonic".to_string()))
    );
}

#[test]
fn helper_errors_are_deterministic() {
    let div_zero = ops::div_int(RuntimeValue::Int(9), RuntimeValue::Int(0), 44)
        .expect_err("division by zero should fail deterministically");
    assert_eq!(div_zero.to_string(), "division by zero at offset 44");

    let bad_add = ops::add_int(
        RuntimeValue::String("x".to_string()),
        RuntimeValue::Int(1),
        45,
    )
    .expect_err("non-int add should fail deterministically");
    assert_eq!(
        bad_add.to_string(),
        "int operator expects int operands, found string at offset 45"
    );

    let missing = collections::map_update(
        RuntimeValue::Map(Vec::new()),
        RuntimeValue::Atom("missing".to_string()),
        RuntimeValue::Int(1),
        46,
    )
    .expect_err("map_update missing key should fail deterministically");
    assert_eq!(
        missing.to_string(),
        "key :missing not found in map at offset 46"
    );

    let bad_builtin = evaluate_builtin_call("map_empty", vec![RuntimeValue::Int(1)], 47)
        .expect_err("builtin arity mismatch should fail deterministically");
    assert_eq!(
        bad_builtin.to_string(),
        "arity mismatch for runtime builtin map_empty: expected 0 args, found 1 at offset 47"
    );
}

#[test]
fn boundary_entrypoints_are_callable_via_abi() {
    let _add_sig: extern "C" fn(TCallContext) -> crate::native_abi::TCallResult = tonic_rt_add_int;
    let _cmp_sig: extern "C" fn(TCallContext) -> crate::native_abi::TCallResult =
        tonic_rt_cmp_int_eq;
    let _map_put_sig: extern "C" fn(TCallContext) -> crate::native_abi::TCallResult =
        tonic_rt_map_put;
    let _host_call_sig: extern "C" fn(TCallContext) -> crate::native_abi::TCallResult =
        tonic_rt_host_call;
    let _protocol_dispatch_sig: extern "C" fn(TCallContext) -> crate::native_abi::TCallResult =
        tonic_rt_protocol_dispatch;

    let add_args = [
        runtime_to_tvalue(RuntimeValue::Int(20)).expect("encode arg"),
        runtime_to_tvalue(RuntimeValue::Int(22)).expect("encode arg"),
    ];
    let add_result = tonic_rt_add_int(TCallContext::from_slice(&add_args));
    assert_eq!(add_result.status, TCallStatus::Ok);
    assert_eq!(
        tvalue_to_runtime(add_result.value).expect("decode add result"),
        RuntimeValue::Int(42)
    );

    let cmp_args = [
        runtime_to_tvalue(RuntimeValue::Int(5)).expect("encode arg"),
        runtime_to_tvalue(RuntimeValue::Int(5)).expect("encode arg"),
    ];
    let cmp_result = tonic_rt_cmp_int_eq(TCallContext::from_slice(&cmp_args));
    assert_eq!(cmp_result.status, TCallStatus::Ok);
    assert_eq!(
        tvalue_to_runtime(cmp_result.value).expect("decode cmp result"),
        RuntimeValue::Bool(true)
    );

    let base_map = RuntimeValue::Map(vec![(
        RuntimeValue::Atom("name".to_string()),
        RuntimeValue::Int(1),
    )]);
    let map_args = [
        runtime_to_tvalue(base_map).expect("encode map"),
        runtime_to_tvalue(RuntimeValue::Atom("name".to_string())).expect("encode key"),
        runtime_to_tvalue(RuntimeValue::Int(2)).expect("encode value"),
    ];
    let map_result = tonic_rt_map_put(TCallContext::from_slice(&map_args));
    assert_eq!(map_result.status, TCallStatus::Ok);
    assert_eq!(
        tvalue_to_runtime(map_result.value)
            .expect("decode map result")
            .render(),
        "%{:name => 2}"
    );

    let host_args = [
        runtime_to_tvalue(RuntimeValue::Atom("sum_ints".to_string())).expect("encode host key"),
        runtime_to_tvalue(RuntimeValue::Int(20)).expect("encode host arg"),
        runtime_to_tvalue(RuntimeValue::Int(22)).expect("encode host arg"),
    ];
    let host_result = tonic_rt_host_call(TCallContext::from_slice(&host_args));
    assert_eq!(host_result.status, TCallStatus::Ok);
    assert_eq!(
        tvalue_to_runtime(host_result.value).expect("decode host result"),
        RuntimeValue::Int(42)
    );

    let protocol_args = [runtime_to_tvalue(RuntimeValue::Tuple(
        Box::new(RuntimeValue::Int(1)),
        Box::new(RuntimeValue::Int(2)),
    ))
    .expect("encode protocol value")];
    let protocol_result = tonic_rt_protocol_dispatch(TCallContext::from_slice(&protocol_args));
    assert_eq!(protocol_result.status, TCallStatus::Ok);
    assert_eq!(
        tvalue_to_runtime(protocol_result.value).expect("decode protocol result"),
        RuntimeValue::Int(1)
    );

    let unknown_host_args =
        [runtime_to_tvalue(RuntimeValue::Atom("missing".to_string())).expect("encode host key")];
    let unknown_host_result = tonic_rt_host_call(TCallContext::from_slice(&unknown_host_args));
    assert_eq!(unknown_host_result.status, TCallStatus::Err);
    assert_eq!(
        tvalue_to_runtime(unknown_host_result.error)
            .expect("decode host error")
            .render(),
        "\"host error: unknown host function: missing at offset 0\""
    );
}
