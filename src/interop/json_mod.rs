use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use serde_json::{Map as JsonMap, Number, Value as JsonValue};

fn expect_exact_args(
    function: &str,
    args: &[RuntimeValue],
    expected: usize,
) -> Result<(), HostError> {
    if args.len() == expected {
        Ok(())
    } else {
        Err(HostError::new(format!(
            "{} expects exactly {} argument{}, found {}",
            function,
            expected,
            if expected == 1 { "" } else { "s" },
            args.len()
        )))
    }
}

fn runtime_to_json(value: &RuntimeValue) -> Result<JsonValue, HostError> {
    match value {
        RuntimeValue::Nil => Ok(JsonValue::Null),
        RuntimeValue::Bool(b) => Ok(JsonValue::Bool(*b)),
        RuntimeValue::Int(n) => Ok(JsonValue::Number(Number::from(*n))),
        RuntimeValue::Float(s) => {
            let f: f64 = s
                .parse()
                .map_err(|_| HostError::new(format!("Json.encode: invalid float {s}")))?;
            Number::from_f64(f)
                .map(JsonValue::Number)
                .ok_or_else(|| HostError::new(format!("Json.encode: non-finite float {s}")))
        }
        RuntimeValue::String(s) => Ok(JsonValue::String(s.clone())),
        RuntimeValue::Atom(a) => Ok(JsonValue::String(a.clone())),
        RuntimeValue::List(items) => {
            let arr: Result<Vec<JsonValue>, HostError> =
                items.iter().map(runtime_to_json).collect();
            Ok(JsonValue::Array(arr?))
        }
        RuntimeValue::Map(entries) => {
            let mut obj = JsonMap::new();
            for (key, val) in entries {
                let key_str = match key {
                    RuntimeValue::String(s) => s.clone(),
                    RuntimeValue::Atom(a) => a.clone(),
                    other => {
                        return Err(HostError::new(format!(
                            "Json.encode: map key must be string or atom, found {}",
                            host_value_kind(other)
                        )));
                    }
                };
                obj.insert(key_str, runtime_to_json(val)?);
            }
            Ok(JsonValue::Object(obj))
        }
        RuntimeValue::Tuple(a, b) => {
            let arr = vec![runtime_to_json(a)?, runtime_to_json(b)?];
            Ok(JsonValue::Array(arr))
        }
        RuntimeValue::Keyword(entries) => {
            let mut obj = JsonMap::new();
            for (key, val) in entries {
                let key_str = match key {
                    RuntimeValue::Atom(a) => a.clone(),
                    RuntimeValue::String(s) => s.clone(),
                    other => {
                        return Err(HostError::new(format!(
                            "Json.encode: keyword key must be atom or string, found {}",
                            host_value_kind(other)
                        )));
                    }
                };
                obj.insert(key_str, runtime_to_json(val)?);
            }
            Ok(JsonValue::Object(obj))
        }
        other => Err(HostError::new(format!(
            "Json.encode: cannot encode {}",
            host_value_kind(other)
        ))),
    }
}

fn json_to_runtime(value: &JsonValue) -> RuntimeValue {
    match value {
        JsonValue::Null => RuntimeValue::Nil,
        JsonValue::Bool(b) => RuntimeValue::Bool(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                RuntimeValue::Int(i)
            } else {
                RuntimeValue::Float(n.to_string())
            }
        }
        JsonValue::String(s) => RuntimeValue::String(s.clone()),
        JsonValue::Array(arr) => RuntimeValue::List(arr.iter().map(json_to_runtime).collect()),
        JsonValue::Object(obj) => {
            let entries: Vec<(RuntimeValue, RuntimeValue)> = obj
                .iter()
                .map(|(k, v)| (RuntimeValue::String(k.clone()), json_to_runtime(v)))
                .collect();
            RuntimeValue::Map(entries)
        }
    }
}

fn host_json_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Json.encode", args, 1)?;
    let json = runtime_to_json(&args[0])?;
    Ok(RuntimeValue::String(json.to_string()))
}

fn host_json_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Json.decode", args, 1)?;
    let s = match &args[0] {
        RuntimeValue::String(s) => s,
        other => {
            return Err(HostError::new(format!(
                "Json.decode expects a string argument, found {}",
                host_value_kind(other)
            )));
        }
    };
    let parsed: JsonValue =
        serde_json::from_str(s).map_err(|e| HostError::new(format!("Json.decode: {e}")))?;
    Ok(json_to_runtime(&parsed))
}

fn host_json_encode_pretty(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Json.encode_pretty", args, 1)?;
    let json = runtime_to_json(&args[0])?;
    let pretty = serde_json::to_string_pretty(&json)
        .map_err(|e| HostError::new(format!("Json.encode_pretty: {e}")))?;
    Ok(RuntimeValue::String(pretty))
}

pub fn register_json_host_functions(registry: &HostRegistry) {
    registry.register("json_encode", host_json_encode);
    registry.register("json_decode", host_json_decode);
    registry.register("json_encode_pretty", host_json_encode_pretty);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn json_encode_string() {
        let result = HOST_REGISTRY
            .call("json_encode", &[s("hello")])
            .expect("json_encode should succeed");
        assert_eq!(result, s("\"hello\""));
    }

    #[test]
    fn json_encode_int() {
        let result = HOST_REGISTRY
            .call("json_encode", &[RuntimeValue::Int(42)])
            .expect("json_encode should succeed");
        assert_eq!(result, s("42"));
    }

    #[test]
    fn json_encode_bool() {
        let result = HOST_REGISTRY
            .call("json_encode", &[RuntimeValue::Bool(true)])
            .expect("json_encode should succeed");
        assert_eq!(result, s("true"));
    }

    #[test]
    fn json_encode_nil() {
        let result = HOST_REGISTRY
            .call("json_encode", &[RuntimeValue::Nil])
            .expect("json_encode should succeed");
        assert_eq!(result, s("null"));
    }

    #[test]
    fn json_encode_list() {
        let list = RuntimeValue::List(vec![
            RuntimeValue::Int(1),
            RuntimeValue::Int(2),
            RuntimeValue::Int(3),
        ]);
        let result = HOST_REGISTRY
            .call("json_encode", &[list])
            .expect("json_encode should succeed");
        assert_eq!(result, s("[1,2,3]"));
    }

    #[test]
    fn json_encode_map() {
        let map = RuntimeValue::Map(vec![(
            RuntimeValue::String("name".to_string()),
            RuntimeValue::String("alice".to_string()),
        )]);
        let result = HOST_REGISTRY
            .call("json_encode", &[map])
            .expect("json_encode should succeed");
        assert_eq!(result, s(r#"{"name":"alice"}"#));
    }

    #[test]
    fn json_encode_atom_keys() {
        let map = RuntimeValue::Map(vec![(
            RuntimeValue::Atom("status".to_string()),
            RuntimeValue::String("ok".to_string()),
        )]);
        let result = HOST_REGISTRY
            .call("json_encode", &[map])
            .expect("json_encode should succeed");
        assert_eq!(result, s(r#"{"status":"ok"}"#));
    }

    #[test]
    fn json_decode_object() {
        let result = HOST_REGISTRY
            .call("json_decode", &[s(r#"{"name":"bob","age":30}"#)])
            .expect("json_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 2);
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn json_decode_array() {
        let result = HOST_REGISTRY
            .call("json_decode", &[s("[1,2,3]")])
            .expect("json_decode should succeed");
        assert_eq!(
            result,
            RuntimeValue::List(vec![
                RuntimeValue::Int(1),
                RuntimeValue::Int(2),
                RuntimeValue::Int(3),
            ])
        );
    }

    #[test]
    fn json_decode_null() {
        let result = HOST_REGISTRY
            .call("json_decode", &[s("null")])
            .expect("json_decode should succeed");
        assert_eq!(result, RuntimeValue::Nil);
    }

    #[test]
    fn json_roundtrip_nested() {
        let original = RuntimeValue::Map(vec![
            (
                RuntimeValue::String("users".to_string()),
                RuntimeValue::List(vec![RuntimeValue::Map(vec![
                    (
                        RuntimeValue::String("name".to_string()),
                        RuntimeValue::String("alice".to_string()),
                    ),
                    (
                        RuntimeValue::String("active".to_string()),
                        RuntimeValue::Bool(true),
                    ),
                ])]),
            ),
            (
                RuntimeValue::String("count".to_string()),
                RuntimeValue::Int(1),
            ),
        ]);

        let encoded = HOST_REGISTRY
            .call("json_encode", &[original])
            .expect("encode should succeed");

        let decoded = HOST_REGISTRY
            .call("json_decode", &[encoded])
            .expect("decode should succeed");

        // Re-encode to verify structural equivalence (map key order may differ)
        let re_encoded = HOST_REGISTRY
            .call("json_encode", &[decoded])
            .expect("re-encode should succeed");

        // Parse both as serde_json::Value to compare order-independently
        let RuntimeValue::String(ref first) = HOST_REGISTRY
            .call(
                "json_encode",
                &[RuntimeValue::Map(vec![
                    (
                        RuntimeValue::String("users".to_string()),
                        RuntimeValue::List(vec![RuntimeValue::Map(vec![
                            (
                                RuntimeValue::String("name".to_string()),
                                RuntimeValue::String("alice".to_string()),
                            ),
                            (
                                RuntimeValue::String("active".to_string()),
                                RuntimeValue::Bool(true),
                            ),
                        ])]),
                    ),
                    (
                        RuntimeValue::String("count".to_string()),
                        RuntimeValue::Int(1),
                    ),
                ])],
            )
            .unwrap()
        else {
            panic!("expected string");
        };
        let RuntimeValue::String(ref second) = re_encoded else {
            panic!("expected string");
        };

        let a: serde_json::Value = serde_json::from_str(first).unwrap();
        let b: serde_json::Value = serde_json::from_str(second).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn json_decode_invalid_returns_error() {
        let err = HOST_REGISTRY
            .call("json_decode", &[s("{invalid")])
            .expect_err("json_decode should fail on invalid JSON");
        assert!(
            err.to_string().contains("Json.decode"),
            "error should mention Json.decode: {err}"
        );
    }

    #[test]
    fn json_encode_pretty_formats_with_indentation() {
        let map = RuntimeValue::Map(vec![(
            RuntimeValue::String("key".to_string()),
            RuntimeValue::Int(1),
        )]);
        let result = HOST_REGISTRY
            .call("json_encode_pretty", &[map])
            .expect("json_encode_pretty should succeed");
        let RuntimeValue::String(ref text) = result else {
            panic!("expected string");
        };
        assert!(text.contains('\n'), "pretty output should contain newlines");
        assert!(
            text.contains("  "),
            "pretty output should contain indentation"
        );
    }
}
