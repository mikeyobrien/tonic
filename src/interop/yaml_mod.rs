use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use serde_yaml::Value as YamlValue;

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

fn runtime_to_yaml(value: &RuntimeValue) -> Result<YamlValue, HostError> {
    match value {
        RuntimeValue::Nil => Ok(YamlValue::Null),
        RuntimeValue::Bool(b) => Ok(YamlValue::Bool(*b)),
        RuntimeValue::Int(n) => Ok(YamlValue::Number(serde_yaml::Number::from(*n))),
        RuntimeValue::Float(s) => {
            let f: f64 = s
                .parse()
                .map_err(|_| HostError::new(format!("Yaml.encode: invalid float {s}")))?;
            Ok(YamlValue::Number(serde_yaml::Number::from(f)))
        }
        RuntimeValue::String(s) => Ok(YamlValue::String(s.clone())),
        RuntimeValue::Atom(a) => Ok(YamlValue::String(a.clone())),
        RuntimeValue::List(items) => {
            let arr: Result<Vec<YamlValue>, HostError> =
                items.iter().map(runtime_to_yaml).collect();
            Ok(YamlValue::Sequence(arr?))
        }
        RuntimeValue::Map(entries) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (key, val) in entries {
                let key_yaml = match key {
                    RuntimeValue::String(s) => YamlValue::String(s.clone()),
                    RuntimeValue::Atom(a) => YamlValue::String(a.clone()),
                    other => {
                        return Err(HostError::new(format!(
                            "Yaml.encode: map key must be string or atom, found {}",
                            host_value_kind(other)
                        )));
                    }
                };
                mapping.insert(key_yaml, runtime_to_yaml(val)?);
            }
            Ok(YamlValue::Mapping(mapping))
        }
        RuntimeValue::Tuple(a, b) => {
            let arr = vec![runtime_to_yaml(a)?, runtime_to_yaml(b)?];
            Ok(YamlValue::Sequence(arr))
        }
        RuntimeValue::Keyword(entries) => {
            let mut mapping = serde_yaml::Mapping::new();
            for (key, val) in entries {
                let key_yaml = match key {
                    RuntimeValue::Atom(a) => YamlValue::String(a.clone()),
                    RuntimeValue::String(s) => YamlValue::String(s.clone()),
                    other => {
                        return Err(HostError::new(format!(
                            "Yaml.encode: keyword key must be atom or string, found {}",
                            host_value_kind(other)
                        )));
                    }
                };
                mapping.insert(key_yaml, runtime_to_yaml(val)?);
            }
            Ok(YamlValue::Mapping(mapping))
        }
        other => Err(HostError::new(format!(
            "Yaml.encode: cannot encode {}",
            host_value_kind(other)
        ))),
    }
}

fn yaml_to_runtime(value: &YamlValue) -> RuntimeValue {
    match value {
        YamlValue::Null => RuntimeValue::Nil,
        YamlValue::Bool(b) => RuntimeValue::Bool(*b),
        YamlValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                RuntimeValue::Int(i)
            } else if let Some(f) = n.as_f64() {
                RuntimeValue::Float(f.to_string())
            } else {
                RuntimeValue::String(n.to_string())
            }
        }
        YamlValue::String(s) => RuntimeValue::String(s.clone()),
        YamlValue::Sequence(arr) => RuntimeValue::List(arr.iter().map(yaml_to_runtime).collect()),
        YamlValue::Mapping(mapping) => {
            let entries: Vec<(RuntimeValue, RuntimeValue)> = mapping
                .iter()
                .map(|(k, v)| {
                    let key = match k {
                        YamlValue::String(s) => RuntimeValue::String(s.clone()),
                        other => RuntimeValue::String(format!("{:?}", other)),
                    };
                    (key, yaml_to_runtime(v))
                })
                .collect();
            RuntimeValue::Map(entries)
        }
        YamlValue::Tagged(tagged) => yaml_to_runtime(&tagged.value),
    }
}

fn host_yaml_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Yaml.encode", args, 1)?;
    let yaml_val = runtime_to_yaml(&args[0])?;
    let output = serde_yaml::to_string(&yaml_val)
        .map_err(|e| HostError::new(format!("Yaml.encode: {e}")))?;
    Ok(RuntimeValue::String(output))
}

fn host_yaml_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Yaml.decode", args, 1)?;
    let s = match &args[0] {
        RuntimeValue::String(s) => s,
        other => {
            return Err(HostError::new(format!(
                "Yaml.decode expects a string argument, found {}",
                host_value_kind(other)
            )));
        }
    };
    let parsed: YamlValue =
        serde_yaml::from_str(s).map_err(|e| HostError::new(format!("Yaml.decode: {e}")))?;
    Ok(yaml_to_runtime(&parsed))
}

pub fn register_yaml_host_functions(registry: &HostRegistry) {
    registry.register("yaml_encode", host_yaml_encode);
    registry.register("yaml_decode", host_yaml_decode);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn yaml_decode_simple_mapping() {
        let input = s("name: alice\nage: 30\n");
        let result = HOST_REGISTRY
            .call("yaml_decode", &[input])
            .expect("yaml_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 2);
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn yaml_decode_nested_mapping() {
        let input = s("server:\n  host: localhost\n  port: 8080\n");
        let result = HOST_REGISTRY
            .call("yaml_decode", &[input])
            .expect("yaml_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 1);
                let (key, val) = &entries[0];
                assert_eq!(key, &s("server"));
                match val {
                    RuntimeValue::Map(inner) => assert_eq!(inner.len(), 2),
                    other => panic!("expected map, got {:?}", other),
                }
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn yaml_decode_sequence() {
        let input = s("items:\n  - 1\n  - 2\n  - 3\n");
        let result = HOST_REGISTRY
            .call("yaml_decode", &[input])
            .expect("yaml_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 1);
                let (_, val) = &entries[0];
                match val {
                    RuntimeValue::List(items) => {
                        assert_eq!(items.len(), 3);
                        assert_eq!(items[0], RuntimeValue::Int(1));
                    }
                    other => panic!("expected list, got {:?}", other),
                }
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn yaml_decode_scalars() {
        let input = s("str: hello\nint: 42\nfloat: 3.14\nbool: true\nnull_val: null\n");
        let result = HOST_REGISTRY
            .call("yaml_decode", &[input])
            .expect("yaml_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 5);
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn yaml_decode_null_becomes_nil() {
        let input = s("value: null\n");
        let result = HOST_REGISTRY
            .call("yaml_decode", &[input])
            .expect("yaml_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 1);
                assert_eq!(entries[0].1, RuntimeValue::Nil);
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn yaml_decode_invalid_returns_error() {
        let err = HOST_REGISTRY
            .call("yaml_decode", &[s(":\n  :\n    - {{")])
            .expect_err("yaml_decode should fail on invalid YAML");
        assert!(
            err.to_string().contains("Yaml.decode"),
            "error should mention Yaml.decode: {err}"
        );
    }

    #[test]
    fn yaml_encode_simple_map() {
        let map = RuntimeValue::Map(vec![
            (s("name"), s("alice")),
            (s("age"), RuntimeValue::Int(30)),
        ]);
        let result = HOST_REGISTRY
            .call("yaml_encode", &[map])
            .expect("yaml_encode should succeed");
        let RuntimeValue::String(ref text) = result else {
            panic!("expected string");
        };
        assert!(text.contains("name"));
        assert!(text.contains("alice"));
    }

    #[test]
    fn yaml_encode_nil_becomes_null() {
        let map = RuntimeValue::Map(vec![(s("value"), RuntimeValue::Nil)]);
        let result = HOST_REGISTRY
            .call("yaml_encode", &[map])
            .expect("yaml_encode should succeed");
        let RuntimeValue::String(ref text) = result else {
            panic!("expected string");
        };
        assert!(text.contains("null"), "nil should encode as null: {text}");
    }

    #[test]
    fn yaml_encode_atom_keys() {
        let map = RuntimeValue::Map(vec![(RuntimeValue::Atom("status".to_string()), s("ok"))]);
        let result = HOST_REGISTRY
            .call("yaml_encode", &[map])
            .expect("yaml_encode should succeed");
        let RuntimeValue::String(ref text) = result else {
            panic!("expected string");
        };
        assert!(text.contains("status"), "should contain atom key: {text}");
    }

    #[test]
    fn yaml_roundtrip() {
        let original = RuntimeValue::Map(vec![
            (s("title"), s("My Config")),
            (s("debug"), RuntimeValue::Bool(false)),
            (s("port"), RuntimeValue::Int(3000)),
        ]);

        let encoded = HOST_REGISTRY
            .call("yaml_encode", &[original])
            .expect("encode should succeed");

        let decoded = HOST_REGISTRY
            .call("yaml_decode", &[encoded])
            .expect("decode should succeed");

        match &decoded {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 3);
                let titles: Vec<_> = entries
                    .iter()
                    .filter(|(k, _)| matches!(k, RuntimeValue::String(s) if s == "title"))
                    .collect();
                assert_eq!(titles.len(), 1);
                assert_eq!(titles[0].1, s("My Config"));
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn yaml_decode_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("yaml_decode", &[RuntimeValue::Int(42)])
            .expect_err("yaml_decode should fail on non-string");
        assert!(
            err.to_string().contains("string"),
            "error should mention string: {err}"
        );
    }
}
