use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

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

fn expect_map_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<Vec<(RuntimeValue, RuntimeValue)>, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::Map(entries) => Ok(entries.clone()),
        other => Err(HostError::new(format!(
            "{} expects map argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_list_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<Vec<RuntimeValue>, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::List(items) => Ok(items.clone()),
        other => Err(HostError::new(format!(
            "{} expects list argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn host_map_keys(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.keys", args, 1)?;
    let entries = expect_map_arg("Map.keys", args, 0)?;
    let keys: Vec<RuntimeValue> = entries.into_iter().map(|(k, _)| k).collect();
    Ok(RuntimeValue::List(keys))
}

fn host_map_values(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.values", args, 1)?;
    let entries = expect_map_arg("Map.values", args, 0)?;
    let values: Vec<RuntimeValue> = entries.into_iter().map(|(_, v)| v).collect();
    Ok(RuntimeValue::List(values))
}

fn host_map_merge(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.merge", args, 2)?;
    let mut base = expect_map_arg("Map.merge", args, 0)?;
    let overrides = expect_map_arg("Map.merge", args, 1)?;

    for (key, value) in overrides {
        if let Some(existing) = base.iter_mut().find(|(k, _)| k == &key) {
            existing.1 = value;
        } else {
            base.push((key, value));
        }
    }

    Ok(RuntimeValue::Map(base))
}

fn host_map_drop(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.drop", args, 2)?;
    let entries = expect_map_arg("Map.drop", args, 0)?;
    let keys = expect_list_arg("Map.drop", args, 1)?;
    let filtered: Vec<(RuntimeValue, RuntimeValue)> = entries
        .into_iter()
        .filter(|(k, _)| !keys.contains(k))
        .collect();
    Ok(RuntimeValue::Map(filtered))
}

fn host_map_take(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.take", args, 2)?;
    let entries = expect_map_arg("Map.take", args, 0)?;
    let keys = expect_list_arg("Map.take", args, 1)?;
    let filtered: Vec<(RuntimeValue, RuntimeValue)> = entries
        .into_iter()
        .filter(|(k, _)| keys.contains(k))
        .collect();
    Ok(RuntimeValue::Map(filtered))
}

fn host_map_has_key(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.has_key?", args, 2)?;
    let entries = expect_map_arg("Map.has_key?", args, 0)?;
    let key = args[1].clone();
    let found = entries.iter().any(|(k, _)| k == &key);
    Ok(RuntimeValue::Bool(found))
}

fn host_map_get(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.get", args, 3)?;
    let entries = expect_map_arg("Map.get", args, 0)?;
    let key = args[1].clone();
    let default = args[2].clone();
    let value = entries
        .into_iter()
        .find(|(k, _)| k == &key)
        .map(|(_, v)| v)
        .unwrap_or(default);
    Ok(value)
}

fn host_map_put(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.put", args, 3)?;
    let mut entries = expect_map_arg("Map.put", args, 0)?;
    let key = args[1].clone();
    let value = args[2].clone();

    if let Some(existing) = entries.iter_mut().find(|(k, _)| k == &key) {
        existing.1 = value;
    } else {
        entries.push((key, value));
    }

    Ok(RuntimeValue::Map(entries))
}

fn host_map_delete(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.delete", args, 2)?;
    let entries = expect_map_arg("Map.delete", args, 0)?;
    let key = args[1].clone();
    let filtered: Vec<(RuntimeValue, RuntimeValue)> =
        entries.into_iter().filter(|(k, _)| k != &key).collect();
    Ok(RuntimeValue::Map(filtered))
}

fn host_map_filter(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.filter", args, 2)?;
    let _entries = expect_map_arg("Map.filter", args, 0)?;
    let fun = match args.get(1) {
        Some(RuntimeValue::Closure(_)) => args[1].clone(),
        Some(other) => {
            return Err(HostError::new(format!(
                "Map.filter expects function argument 2; found {}",
                host_value_kind(other)
            )));
        }
        None => {
            return Err(HostError::new("Map.filter missing required argument 2"));
        }
    };

    let _ = fun;
    // Closures require runtime invocation — return an unsupported error so callers know
    Err(HostError::new(
        "Map.filter with closures requires runtime dispatch; use a for comprehension instead",
    ))
}

fn host_map_reject(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Map.reject", args, 2)?;
    let _ = expect_map_arg("Map.reject", args, 0)?;
    let _ = match args.get(1) {
        Some(RuntimeValue::Closure(_)) => args[1].clone(),
        Some(other) => {
            return Err(HostError::new(format!(
                "Map.reject expects function argument 2; found {}",
                host_value_kind(other)
            )));
        }
        None => {
            return Err(HostError::new("Map.reject missing required argument 2"));
        }
    };

    Err(HostError::new(
        "Map.reject with closures requires runtime dispatch; use a for comprehension instead",
    ))
}

pub fn register_map_host_functions(registry: &HostRegistry) {
    registry.register("map_keys", host_map_keys);
    registry.register("map_values", host_map_values);
    registry.register("map_merge", host_map_merge);
    registry.register("map_drop", host_map_drop);
    registry.register("map_take", host_map_take);
    registry.register("map_has_key", host_map_has_key);
    registry.register("map_get", host_map_get);
    registry.register("map_put", host_map_put);
    registry.register("map_delete", host_map_delete);
    registry.register("map_filter", host_map_filter);
    registry.register("map_reject", host_map_reject);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn atom(s: &str) -> RuntimeValue {
        RuntimeValue::Atom(s.to_string())
    }

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    fn i(n: i64) -> RuntimeValue {
        RuntimeValue::Int(n)
    }

    fn map(entries: Vec<(&str, RuntimeValue)>) -> RuntimeValue {
        RuntimeValue::Map(entries.into_iter().map(|(k, v)| (atom(k), v)).collect())
    }

    #[test]
    fn map_keys_returns_key_list() {
        let m = map(vec![("a", i(1)), ("b", i(2))]);
        let result = HOST_REGISTRY
            .call("map_keys", &[m])
            .expect("map_keys should succeed");
        assert_eq!(result, RuntimeValue::List(vec![atom("a"), atom("b")]));
    }

    #[test]
    fn map_values_returns_value_list() {
        let m = map(vec![("a", i(1)), ("b", i(2))]);
        let result = HOST_REGISTRY
            .call("map_values", &[m])
            .expect("map_values should succeed");
        assert_eq!(result, RuntimeValue::List(vec![i(1), i(2)]));
    }

    #[test]
    fn map_merge_overrides_existing_keys() {
        let base = map(vec![("a", i(1)), ("b", i(2))]);
        let overrides = map(vec![("b", i(99)), ("c", i(3))]);
        let result = HOST_REGISTRY
            .call("map_merge", &[base, overrides])
            .expect("map_merge should succeed");
        assert_eq!(
            result,
            RuntimeValue::Map(vec![
                (atom("a"), i(1)),
                (atom("b"), i(99)),
                (atom("c"), i(3)),
            ])
        );
    }

    #[test]
    fn map_drop_removes_specified_keys() {
        let m = map(vec![("a", i(1)), ("b", i(2)), ("c", i(3))]);
        let keys = RuntimeValue::List(vec![atom("a"), atom("c")]);
        let result = HOST_REGISTRY
            .call("map_drop", &[m, keys])
            .expect("map_drop should succeed");
        assert_eq!(result, RuntimeValue::Map(vec![(atom("b"), i(2))]));
    }

    #[test]
    fn map_take_keeps_specified_keys() {
        let m = map(vec![("a", i(1)), ("b", i(2)), ("c", i(3))]);
        let keys = RuntimeValue::List(vec![atom("a"), atom("c")]);
        let result = HOST_REGISTRY
            .call("map_take", &[m, keys])
            .expect("map_take should succeed");
        assert_eq!(
            result,
            RuntimeValue::Map(vec![(atom("a"), i(1)), (atom("c"), i(3))])
        );
    }

    #[test]
    fn map_has_key_finds_existing_key() {
        let m = map(vec![("a", i(1))]);
        let yes = HOST_REGISTRY
            .call("map_has_key", &[m.clone(), atom("a")])
            .expect("map_has_key should succeed");
        assert_eq!(yes, RuntimeValue::Bool(true));

        let no = HOST_REGISTRY
            .call("map_has_key", &[m, atom("z")])
            .expect("map_has_key should succeed for missing key");
        assert_eq!(no, RuntimeValue::Bool(false));
    }

    #[test]
    fn map_get_returns_value_or_default() {
        let m = map(vec![("a", i(1))]);
        let found = HOST_REGISTRY
            .call("map_get", &[m.clone(), atom("a"), RuntimeValue::Nil])
            .expect("map_get should succeed");
        assert_eq!(found, i(1));

        let default_val = HOST_REGISTRY
            .call("map_get", &[m, atom("z"), s("default")])
            .expect("map_get should succeed for missing key");
        assert_eq!(default_val, s("default"));
    }

    #[test]
    fn map_put_inserts_and_updates() {
        let m = map(vec![("a", i(1))]);
        let inserted = HOST_REGISTRY
            .call("map_put", &[m.clone(), atom("b"), i(2)])
            .expect("map_put should succeed");
        assert_eq!(
            inserted,
            RuntimeValue::Map(vec![(atom("a"), i(1)), (atom("b"), i(2))])
        );

        let updated = HOST_REGISTRY
            .call("map_put", &[m, atom("a"), i(99)])
            .expect("map_put should succeed for update");
        assert_eq!(updated, RuntimeValue::Map(vec![(atom("a"), i(99))]));
    }

    #[test]
    fn map_delete_removes_key() {
        let m = map(vec![("a", i(1)), ("b", i(2))]);
        let result = HOST_REGISTRY
            .call("map_delete", &[m, atom("a")])
            .expect("map_delete should succeed");
        assert_eq!(result, RuntimeValue::Map(vec![(atom("b"), i(2))]));
    }
}
