use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;
use toml::Value as TomlValue;

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

fn runtime_to_toml(value: &RuntimeValue) -> Result<TomlValue, HostError> {
    match value {
        RuntimeValue::Bool(b) => Ok(TomlValue::Boolean(*b)),
        RuntimeValue::Int(n) => Ok(TomlValue::Integer(*n)),
        RuntimeValue::Float(s) => {
            let f: f64 = s
                .parse()
                .map_err(|_| HostError::new(format!("Toml.encode: invalid float {s}")))?;
            Ok(TomlValue::Float(f))
        }
        RuntimeValue::String(s) => Ok(TomlValue::String(s.clone())),
        RuntimeValue::Atom(a) => Ok(TomlValue::String(a.clone())),
        RuntimeValue::List(items) => {
            let arr: Result<Vec<TomlValue>, HostError> =
                items.iter().map(runtime_to_toml).collect();
            Ok(TomlValue::Array(arr?))
        }
        RuntimeValue::Map(entries) => {
            let mut table = toml::map::Map::new();
            for (key, val) in entries {
                let key_str = match key {
                    RuntimeValue::String(s) => s.clone(),
                    RuntimeValue::Atom(a) => a.clone(),
                    other => {
                        return Err(HostError::new(format!(
                            "Toml.encode: map key must be string or atom, found {}",
                            host_value_kind(other)
                        )));
                    }
                };
                table.insert(key_str, runtime_to_toml(val)?);
            }
            Ok(TomlValue::Table(table))
        }
        RuntimeValue::Keyword(entries) => {
            let mut table = toml::map::Map::new();
            for (key, val) in entries {
                let key_str = match key {
                    RuntimeValue::Atom(a) => a.clone(),
                    RuntimeValue::String(s) => s.clone(),
                    other => {
                        return Err(HostError::new(format!(
                            "Toml.encode: keyword key must be atom or string, found {}",
                            host_value_kind(other)
                        )));
                    }
                };
                table.insert(key_str, runtime_to_toml(val)?);
            }
            Ok(TomlValue::Table(table))
        }
        RuntimeValue::Nil => Err(HostError::new(
            "Toml.encode: nil cannot be represented in TOML",
        )),
        other => Err(HostError::new(format!(
            "Toml.encode: cannot encode {}",
            host_value_kind(other)
        ))),
    }
}

fn toml_to_runtime(value: &TomlValue) -> RuntimeValue {
    match value {
        TomlValue::Boolean(b) => RuntimeValue::Bool(*b),
        TomlValue::Integer(n) => RuntimeValue::Int(*n),
        TomlValue::Float(f) => RuntimeValue::Float(f.to_string()),
        TomlValue::String(s) => RuntimeValue::String(s.clone()),
        TomlValue::Array(arr) => RuntimeValue::List(arr.iter().map(toml_to_runtime).collect()),
        TomlValue::Table(table) => {
            let entries: Vec<(RuntimeValue, RuntimeValue)> = table
                .iter()
                .map(|(k, v)| (RuntimeValue::String(k.clone()), toml_to_runtime(v)))
                .collect();
            RuntimeValue::Map(entries)
        }
        TomlValue::Datetime(dt) => RuntimeValue::String(dt.to_string()),
    }
}

fn host_toml_encode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Toml.encode", args, 1)?;
    let toml_val = runtime_to_toml(&args[0])?;
    let output =
        toml::to_string(&toml_val).map_err(|e| HostError::new(format!("Toml.encode: {e}")))?;
    Ok(RuntimeValue::String(output))
}

fn host_toml_decode(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Toml.decode", args, 1)?;
    let s = match &args[0] {
        RuntimeValue::String(s) => s,
        other => {
            return Err(HostError::new(format!(
                "Toml.decode expects a string argument, found {}",
                host_value_kind(other)
            )));
        }
    };
    let parsed: TomlValue =
        toml::from_str(s).map_err(|e| HostError::new(format!("Toml.decode: {e}")))?;
    Ok(toml_to_runtime(&parsed))
}

pub fn register_toml_host_functions(registry: &HostRegistry) {
    registry.register("toml_encode", host_toml_encode);
    registry.register("toml_decode", host_toml_decode);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn toml_decode_simple_table() {
        let input = s("[server]\nhost = \"localhost\"\nport = 8080\n");
        let result = HOST_REGISTRY
            .call("toml_decode", &[input])
            .expect("toml_decode should succeed");
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
    fn toml_decode_scalars() {
        let input = s("name = \"alice\"\nage = 30\nactive = true\nscore = 3.14\n");
        let result = HOST_REGISTRY
            .call("toml_decode", &[input])
            .expect("toml_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 4);
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn toml_decode_array() {
        let input = s("values = [1, 2, 3]\n");
        let result = HOST_REGISTRY
            .call("toml_decode", &[input])
            .expect("toml_decode should succeed");
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
    fn toml_decode_invalid_returns_error() {
        let err = HOST_REGISTRY
            .call("toml_decode", &[s("[invalid")])
            .expect_err("toml_decode should fail on invalid TOML");
        assert!(
            err.to_string().contains("Toml.decode"),
            "error should mention Toml.decode: {err}"
        );
    }

    #[test]
    fn toml_encode_simple_map() {
        let map = RuntimeValue::Map(vec![
            (s("name"), s("alice")),
            (s("age"), RuntimeValue::Int(30)),
        ]);
        let result = HOST_REGISTRY
            .call("toml_encode", &[map])
            .expect("toml_encode should succeed");
        let RuntimeValue::String(ref text) = result else {
            panic!("expected string");
        };
        assert!(text.contains("name"));
        assert!(text.contains("alice"));
    }

    #[test]
    fn toml_encode_nested_table() {
        let inner = RuntimeValue::Map(vec![(s("host"), s("localhost"))]);
        let outer = RuntimeValue::Map(vec![(s("server"), inner)]);
        let result = HOST_REGISTRY
            .call("toml_encode", &[outer])
            .expect("toml_encode should succeed");
        let RuntimeValue::String(ref text) = result else {
            panic!("expected string");
        };
        assert!(
            text.contains("[server]"),
            "should contain table header: {text}"
        );
        assert!(text.contains("localhost"));
    }

    #[test]
    fn toml_encode_nil_returns_error() {
        let err = HOST_REGISTRY
            .call("toml_encode", &[RuntimeValue::Nil])
            .expect_err("toml_encode should reject nil");
        assert!(
            err.to_string().contains("nil"),
            "error should mention nil: {err}"
        );
    }

    #[test]
    fn toml_roundtrip() {
        let original = RuntimeValue::Map(vec![
            (s("title"), s("My Config")),
            (s("debug"), RuntimeValue::Bool(false)),
            (s("port"), RuntimeValue::Int(3000)),
        ]);

        let encoded = HOST_REGISTRY
            .call("toml_encode", &[original])
            .expect("encode should succeed");

        let decoded = HOST_REGISTRY
            .call("toml_decode", &[encoded])
            .expect("decode should succeed");

        // Verify key fields survived round-trip
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
    fn toml_decode_datetime_as_string() {
        let input = s("created = 2024-01-15T10:30:00Z\n");
        let result = HOST_REGISTRY
            .call("toml_decode", &[input])
            .expect("toml_decode should succeed");
        match &result {
            RuntimeValue::Map(entries) => {
                assert_eq!(entries.len(), 1);
                let (_, val) = &entries[0];
                match val {
                    RuntimeValue::String(dt) => {
                        assert!(dt.contains("2024"), "datetime should contain year: {dt}");
                    }
                    other => panic!("expected string for datetime, got {:?}", other),
                }
            }
            other => panic!("expected map, got {:?}", other),
        }
    }

    #[test]
    fn toml_encode_atom_keys() {
        let map = RuntimeValue::Map(vec![(RuntimeValue::Atom("status".to_string()), s("ok"))]);
        let result = HOST_REGISTRY
            .call("toml_encode", &[map])
            .expect("toml_encode should succeed");
        let RuntimeValue::String(ref text) = result else {
            panic!("expected string");
        };
        assert!(text.contains("status"), "should contain atom key: {text}");
    }

    #[test]
    fn toml_decode_wrong_type_returns_error() {
        let err = HOST_REGISTRY
            .call("toml_decode", &[RuntimeValue::Int(42)])
            .expect_err("toml_decode should fail on non-string");
        assert!(
            err.to_string().contains("string"),
            "error should mention string: {err}"
        );
    }
}
