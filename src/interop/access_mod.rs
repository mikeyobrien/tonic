use super::system::expect_exact_args;
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn ok_tuple(val: RuntimeValue) -> RuntimeValue {
    RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("ok".to_string())),
        Box::new(val),
    )
}

/// Traverse nested maps/lists by a key path, returning the value or nil.
/// Path is a list of keys: strings/atoms for maps, integers for list indices.
fn host_access_get_in(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Access.get_in", args, 2)?;
    let path = match &args[1] {
        RuntimeValue::List(keys) => keys,
        other => {
            return Err(HostError::new(format!(
                "Access.get_in expects a list as path, found {}",
                host_value_kind(other)
            )));
        }
    };

    let mut current = args[0].clone();
    for key in path {
        current = access_step(&current, key);
        if current == RuntimeValue::Nil {
            return Ok(RuntimeValue::Nil);
        }
    }
    Ok(current)
}

/// Set a value at a nested path, creating intermediate maps as needed.
fn host_access_put_in(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Access.put_in", args, 3)?;
    let path = match &args[1] {
        RuntimeValue::List(keys) => keys.clone(),
        other => {
            return Err(HostError::new(format!(
                "Access.put_in expects a list as path, found {}",
                host_value_kind(other)
            )));
        }
    };

    if path.is_empty() {
        return Ok(args[2].clone());
    }

    Ok(put_in_recursive(&args[0], &path, &args[2]))
}

/// Like get_in for a single key, returning {:ok, val} or :error.
fn host_access_fetch(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Access.fetch", args, 2)?;
    let result = access_step(&args[0], &args[1]);
    if result == RuntimeValue::Nil {
        // Distinguish between key-not-found and key-mapped-to-nil
        if key_exists(&args[0], &args[1]) {
            Ok(ok_tuple(RuntimeValue::Nil))
        } else {
            Ok(RuntimeValue::Atom("error".to_string()))
        }
    } else {
        Ok(ok_tuple(result))
    }
}

/// Return the keys of a map.
fn host_access_keys(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Access.keys", args, 1)?;
    match &args[0] {
        RuntimeValue::Map(pairs) => {
            let keys: Vec<RuntimeValue> = pairs.iter().map(|(k, _)| k.clone()).collect();
            Ok(RuntimeValue::List(keys))
        }
        other => Err(HostError::new(format!(
            "Access.keys expects a map, found {}",
            host_value_kind(other)
        ))),
    }
}

/// Single-step access into a map or list.
fn access_step(data: &RuntimeValue, key: &RuntimeValue) -> RuntimeValue {
    match data {
        RuntimeValue::Map(pairs) => {
            for (k, v) in pairs {
                if k == key {
                    return v.clone();
                }
            }
            RuntimeValue::Nil
        }
        RuntimeValue::List(items) => {
            if let RuntimeValue::Int(idx) = key {
                let i = *idx as usize;
                if (*idx >= 0) && i < items.len() {
                    items[i].clone()
                } else {
                    RuntimeValue::Nil
                }
            } else {
                RuntimeValue::Nil
            }
        }
        RuntimeValue::Nil => RuntimeValue::Nil,
        _ => RuntimeValue::Nil,
    }
}

/// Check if a key exists in a map (not just returns nil).
fn key_exists(data: &RuntimeValue, key: &RuntimeValue) -> bool {
    match data {
        RuntimeValue::Map(pairs) => pairs.iter().any(|(k, _)| k == key),
        RuntimeValue::List(items) => {
            if let RuntimeValue::Int(idx) = key {
                let i = *idx as usize;
                *idx >= 0 && i < items.len()
            } else {
                false
            }
        }
        _ => false,
    }
}

/// Recursively build a new structure with the value set at the given path.
fn put_in_recursive(
    data: &RuntimeValue,
    path: &[RuntimeValue],
    value: &RuntimeValue,
) -> RuntimeValue {
    if path.is_empty() {
        return value.clone();
    }

    let key = &path[0];
    let rest = &path[1..];

    match data {
        RuntimeValue::Map(pairs) => {
            let child = access_step(data, key);
            let next = if child == RuntimeValue::Nil && !key_exists(data, key) {
                // Create intermediate map
                put_in_recursive(&RuntimeValue::Map(vec![]), rest, value)
            } else {
                put_in_recursive(&child, rest, value)
            };

            // Replace or add the key
            let mut new_pairs: Vec<(RuntimeValue, RuntimeValue)> = Vec::new();
            let mut replaced = false;
            for (k, v) in pairs {
                if k == key {
                    new_pairs.push((k.clone(), next.clone()));
                    replaced = true;
                } else {
                    new_pairs.push((k.clone(), v.clone()));
                }
            }
            if !replaced {
                new_pairs.push((key.clone(), next));
            }
            RuntimeValue::Map(new_pairs)
        }
        RuntimeValue::List(items) => {
            if let RuntimeValue::Int(idx) = key {
                let i = *idx as usize;
                if *idx >= 0 && i < items.len() {
                    let child = &items[i];
                    let next = put_in_recursive(child, rest, value);
                    let mut new_items = items.clone();
                    new_items[i] = next;
                    RuntimeValue::List(new_items)
                } else {
                    // Out of bounds — return unchanged
                    data.clone()
                }
            } else {
                data.clone()
            }
        }
        RuntimeValue::Nil => {
            // Create intermediate map for nil intermediate
            let inner = put_in_recursive(&RuntimeValue::Map(vec![]), rest, value);
            RuntimeValue::Map(vec![(key.clone(), inner)])
        }
        _ => data.clone(),
    }
}

pub fn register_access_host_functions(registry: &HostRegistry) {
    registry.register("access_get_in", host_access_get_in);
    registry.register("access_put_in", host_access_put_in);
    registry.register("access_fetch", host_access_fetch);
    registry.register("access_keys", host_access_keys);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(val: &str) -> RuntimeValue {
        RuntimeValue::String(val.to_string())
    }

    fn atom(val: &str) -> RuntimeValue {
        RuntimeValue::Atom(val.to_string())
    }

    fn int(val: i64) -> RuntimeValue {
        RuntimeValue::Int(val)
    }

    fn map(pairs: Vec<(RuntimeValue, RuntimeValue)>) -> RuntimeValue {
        RuntimeValue::Map(pairs)
    }

    fn list(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::List(items)
    }

    #[test]
    fn get_in_flat_map() {
        let data = map(vec![(s("a"), int(1)), (s("b"), int(2))]);
        let result = host_access_get_in(&[data, list(vec![s("a")])]).unwrap();
        assert_eq!(result, int(1));
    }

    #[test]
    fn get_in_nested_map() {
        let inner = map(vec![(s("y"), int(42))]);
        let data = map(vec![(s("x"), inner)]);
        let result = host_access_get_in(&[data, list(vec![s("x"), s("y")])]).unwrap();
        assert_eq!(result, int(42));
    }

    #[test]
    fn get_in_with_atom_keys() {
        let data = map(vec![(atom("name"), s("tonic"))]);
        let result = host_access_get_in(&[data, list(vec![atom("name")])]).unwrap();
        assert_eq!(result, s("tonic"));
    }

    #[test]
    fn get_in_missing_key_returns_nil() {
        let data = map(vec![(s("a"), int(1))]);
        let result = host_access_get_in(&[data, list(vec![s("z")])]).unwrap();
        assert_eq!(result, RuntimeValue::Nil);
    }

    #[test]
    fn get_in_nil_intermediate_returns_nil() {
        let data = map(vec![(s("a"), RuntimeValue::Nil)]);
        let result = host_access_get_in(&[data, list(vec![s("a"), s("b")])]).unwrap();
        assert_eq!(result, RuntimeValue::Nil);
    }

    #[test]
    fn get_in_list_index() {
        let data = list(vec![s("zero"), s("one"), s("two")]);
        let result = host_access_get_in(&[data, list(vec![int(1)])]).unwrap();
        assert_eq!(result, s("one"));
    }

    #[test]
    fn get_in_nested_map_and_list() {
        let inner = list(vec![int(10), int(20), int(30)]);
        let data = map(vec![(s("items"), inner)]);
        let result = host_access_get_in(&[data, list(vec![s("items"), int(2)])]).unwrap();
        assert_eq!(result, int(30));
    }

    #[test]
    fn put_in_flat_map() {
        let data = map(vec![(s("a"), int(1))]);
        let result = host_access_put_in(&[data, list(vec![s("b")]), int(2)]).unwrap();
        assert_eq!(result, map(vec![(s("a"), int(1)), (s("b"), int(2))]));
    }

    #[test]
    fn put_in_nested_creates_intermediates() {
        let data = map(vec![]);
        let result = host_access_put_in(&[data, list(vec![s("a"), s("b")]), int(42)]).unwrap();
        let expected = map(vec![(s("a"), map(vec![(s("b"), int(42))]))]);
        assert_eq!(result, expected);
    }

    #[test]
    fn put_in_overwrites_existing() {
        let inner = map(vec![(s("y"), int(1))]);
        let data = map(vec![(s("x"), inner)]);
        let result = host_access_put_in(&[data, list(vec![s("x"), s("y")]), int(99)]).unwrap();
        let expected = map(vec![(s("x"), map(vec![(s("y"), int(99))]))]);
        assert_eq!(result, expected);
    }

    #[test]
    fn put_in_list_index() {
        let data = list(vec![int(10), int(20), int(30)]);
        let result = host_access_put_in(&[data, list(vec![int(1)]), int(99)]).unwrap();
        assert_eq!(result, list(vec![int(10), int(99), int(30)]));
    }

    #[test]
    fn fetch_found() {
        let data = map(vec![(s("a"), int(1))]);
        let result = host_access_fetch(&[data, s("a")]).unwrap();
        assert_eq!(result, ok_tuple(int(1)));
    }

    #[test]
    fn fetch_missing() {
        let data = map(vec![(s("a"), int(1))]);
        let result = host_access_fetch(&[data, s("z")]).unwrap();
        assert_eq!(result, atom("error"));
    }

    #[test]
    fn fetch_nil_value_is_ok() {
        let data = map(vec![(s("a"), RuntimeValue::Nil)]);
        let result = host_access_fetch(&[data, s("a")]).unwrap();
        assert_eq!(result, ok_tuple(RuntimeValue::Nil));
    }

    #[test]
    fn keys_of_map() {
        let data = map(vec![(s("x"), int(1)), (s("y"), int(2))]);
        let result = host_access_keys(&[data]).unwrap();
        assert_eq!(result, list(vec![s("x"), s("y")]));
    }

    #[test]
    fn keys_rejects_non_map() {
        let result = host_access_keys(&[int(42)]);
        assert!(result.is_err());
    }

    #[test]
    fn get_in_rejects_non_list_path() {
        let result = host_access_get_in(&[map(vec![]), s("not_a_list")]);
        assert!(result.is_err());
    }

    #[test]
    fn put_in_rejects_non_list_path() {
        let result = host_access_put_in(&[map(vec![]), s("not_a_list"), int(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn registration_via_call() {
        let registry = HostRegistry::new();
        register_access_host_functions(&registry);
        let data = map(vec![(s("a"), int(1))]);
        let result = registry
            .call("access_get_in", &[data, list(vec![s("a")])])
            .unwrap();
        assert_eq!(result, int(1));
    }
}
