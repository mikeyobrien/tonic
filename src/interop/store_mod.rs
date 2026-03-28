use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use super::system::expect_exact_args;
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

type StoreEntries = Vec<(RuntimeValue, RuntimeValue)>;
type StoreMap = HashMap<String, StoreEntries>;

/// Global store: maps store_id -> Vec<(key, value)> pairs.
/// We use Vec<(RuntimeValue, RuntimeValue)> because RuntimeValue doesn't implement Hash.
static STORES: LazyLock<Mutex<StoreMap>> = LazyLock::new(|| Mutex::new(HashMap::new()));

static STORE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn next_store_id() -> String {
    let id = STORE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("store_{}", id)
}

fn with_store<F, R>(name: &str, store_id: &str, f: F) -> Result<R, HostError>
where
    F: FnOnce(&mut StoreEntries) -> R,
{
    let mut stores = STORES.lock().unwrap();
    match stores.get_mut(store_id) {
        Some(store) => Ok(f(store)),
        None => Err(HostError::new(format!(
            "{}: store '{}' does not exist",
            name, store_id
        ))),
    }
}

fn extract_store_id(name: &str, val: &RuntimeValue) -> Result<String, HostError> {
    match val {
        RuntimeValue::String(s) => Ok(s.clone()),
        other => Err(HostError::new(format!(
            "{} expects a store id (string), found {}",
            name,
            host_value_kind(other)
        ))),
    }
}

fn host_store_new(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.new", args, 0)?;
    let id = next_store_id();
    STORES.lock().unwrap().insert(id.clone(), Vec::new());
    Ok(RuntimeValue::String(id))
}

fn host_store_put(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.put", args, 3)?;
    let store_id = extract_store_id("Store.put", &args[0])?;
    let key = args[1].clone();
    let value = args[2].clone();
    with_store("Store.put", &store_id, |store| {
        // Replace existing key if found
        if let Some(entry) = store.iter_mut().find(|(k, _)| k == &key) {
            entry.1 = value;
        } else {
            store.push((key, value));
        }
    })?;
    Ok(RuntimeValue::Atom("ok".into()))
}

fn host_store_get(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(HostError::new(
            "Store.get expects 2 or 3 arguments (store_id, key[, default])",
        ));
    }
    let store_id = extract_store_id("Store.get", &args[0])?;
    let key = &args[1];
    let default = if args.len() == 3 {
        args[2].clone()
    } else {
        RuntimeValue::Nil
    };
    with_store("Store.get", &store_id, |store| {
        store
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or(default)
    })
}

fn host_store_delete(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.delete", args, 2)?;
    let store_id = extract_store_id("Store.delete", &args[0])?;
    let key = &args[1];
    with_store("Store.delete", &store_id, |store| {
        store.retain(|(k, _)| k != key);
    })?;
    Ok(RuntimeValue::Atom("ok".into()))
}

fn host_store_has_key(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.has_key?", args, 2)?;
    let store_id = extract_store_id("Store.has_key?", &args[0])?;
    let key = &args[1];
    let found = with_store("Store.has_key?", &store_id, |store| {
        store.iter().any(|(k, _)| k == key)
    })?;
    Ok(RuntimeValue::Bool(found))
}

fn host_store_keys(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.keys", args, 1)?;
    let store_id = extract_store_id("Store.keys", &args[0])?;
    let keys = with_store("Store.keys", &store_id, |store| {
        store.iter().map(|(k, _)| k.clone()).collect::<Vec<_>>()
    })?;
    Ok(RuntimeValue::List(keys))
}

fn host_store_values(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.values", args, 1)?;
    let store_id = extract_store_id("Store.values", &args[0])?;
    let values = with_store("Store.values", &store_id, |store| {
        store.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>()
    })?;
    Ok(RuntimeValue::List(values))
}

fn host_store_size(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.size", args, 1)?;
    let store_id = extract_store_id("Store.size", &args[0])?;
    let size = with_store("Store.size", &store_id, |store| store.len() as i64)?;
    Ok(RuntimeValue::Int(size))
}

fn host_store_to_list(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.to_list", args, 1)?;
    let store_id = extract_store_id("Store.to_list", &args[0])?;
    let pairs = with_store("Store.to_list", &store_id, |store| {
        store
            .iter()
            .map(|(k, v)| RuntimeValue::Tuple(Box::new(k.clone()), Box::new(v.clone())))
            .collect::<Vec<_>>()
    })?;
    Ok(RuntimeValue::List(pairs))
}

fn host_store_clear(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.clear", args, 1)?;
    let store_id = extract_store_id("Store.clear", &args[0])?;
    with_store("Store.clear", &store_id, |store| {
        store.clear();
    })?;
    Ok(RuntimeValue::Atom("ok".into()))
}

fn host_store_drop(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Store.drop", args, 1)?;
    let store_id = extract_store_id("Store.drop", &args[0])?;
    let mut stores = STORES.lock().unwrap();
    if stores.remove(&store_id).is_none() {
        return Err(HostError::new(format!(
            "Store.drop: store '{}' does not exist",
            store_id
        )));
    }
    Ok(RuntimeValue::Atom("ok".into()))
}

pub fn register_store_host_functions(registry: &HostRegistry) {
    registry.register("store_new", host_store_new);
    registry.register("store_put", host_store_put);
    registry.register("store_get", host_store_get);
    registry.register("store_delete", host_store_delete);
    registry.register("store_has_key", host_store_has_key);
    registry.register("store_keys", host_store_keys);
    registry.register("store_values", host_store_values);
    registry.register("store_size", host_store_size);
    registry.register("store_to_list", host_store_to_list);
    registry.register("store_clear", host_store_clear);
    registry.register("store_drop", host_store_drop);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex as StdMutex;

    // Serialize tests that share global STORES state
    static TEST_LOCK: LazyLock<StdMutex<()>> = LazyLock::new(|| StdMutex::new(()));

    fn new_store() -> String {
        match host_store_new(&[]).unwrap() {
            RuntimeValue::String(id) => id,
            other => panic!("expected String, got {:?}", other),
        }
    }

    #[test]
    fn store_new_returns_string_id() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        assert!(id.starts_with("store_"));
    }

    #[test]
    fn store_put_get_round_trip() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id.clone());

        let result = host_store_put(&[
            sid.clone(),
            RuntimeValue::Atom("name".into()),
            RuntimeValue::String("Alice".into()),
        ])
        .unwrap();
        assert_eq!(result, RuntimeValue::Atom("ok".into()));

        let val = host_store_get(&[sid.clone(), RuntimeValue::Atom("name".into())]).unwrap();
        assert_eq!(val, RuntimeValue::String("Alice".into()));
    }

    #[test]
    fn store_get_missing_returns_nil() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);
        let val = host_store_get(&[sid, RuntimeValue::Atom("missing".into())]).unwrap();
        assert_eq!(val, RuntimeValue::Nil);
    }

    #[test]
    fn store_get_with_default() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);
        let val = host_store_get(&[
            sid,
            RuntimeValue::Atom("missing".into()),
            RuntimeValue::Int(42),
        ])
        .unwrap();
        assert_eq!(val, RuntimeValue::Int(42));
    }

    #[test]
    fn store_put_overwrites_existing() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);
        let key = RuntimeValue::Atom("x".into());

        host_store_put(&[sid.clone(), key.clone(), RuntimeValue::Int(1)]).unwrap();
        host_store_put(&[sid.clone(), key.clone(), RuntimeValue::Int(2)]).unwrap();

        let val = host_store_get(&[sid, key]).unwrap();
        assert_eq!(val, RuntimeValue::Int(2));

        // Size should be 1, not 2
    }

    #[test]
    fn store_delete_removes_key() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);
        let key = RuntimeValue::Atom("temp".into());

        host_store_put(&[sid.clone(), key.clone(), RuntimeValue::Int(99)]).unwrap();
        host_store_delete(&[sid.clone(), key.clone()]).unwrap();

        let val = host_store_get(&[sid, key]).unwrap();
        assert_eq!(val, RuntimeValue::Nil);
    }

    #[test]
    fn store_has_key_true_and_false() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);
        let key = RuntimeValue::Atom("exists".into());

        let before = host_store_has_key(&[sid.clone(), key.clone()]).unwrap();
        assert_eq!(before, RuntimeValue::Bool(false));

        host_store_put(&[sid.clone(), key.clone(), RuntimeValue::Int(1)]).unwrap();

        let after = host_store_has_key(&[sid, key]).unwrap();
        assert_eq!(after, RuntimeValue::Bool(true));
    }

    #[test]
    fn store_keys_and_values() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);

        host_store_put(&[
            sid.clone(),
            RuntimeValue::Atom("a".into()),
            RuntimeValue::Int(1),
        ])
        .unwrap();
        host_store_put(&[
            sid.clone(),
            RuntimeValue::Atom("b".into()),
            RuntimeValue::Int(2),
        ])
        .unwrap();

        let keys = host_store_keys(std::slice::from_ref(&sid)).unwrap();
        assert_eq!(
            keys,
            RuntimeValue::List(vec![
                RuntimeValue::Atom("a".into()),
                RuntimeValue::Atom("b".into()),
            ])
        );

        let values = host_store_values(&[sid]).unwrap();
        assert_eq!(
            values,
            RuntimeValue::List(vec![RuntimeValue::Int(1), RuntimeValue::Int(2),])
        );
    }

    #[test]
    fn store_size() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);

        assert_eq!(
            host_store_size(std::slice::from_ref(&sid)).unwrap(),
            RuntimeValue::Int(0)
        );

        host_store_put(&[
            sid.clone(),
            RuntimeValue::Atom("x".into()),
            RuntimeValue::Int(1),
        ])
        .unwrap();
        assert_eq!(
            host_store_size(std::slice::from_ref(&sid)).unwrap(),
            RuntimeValue::Int(1)
        );

        host_store_put(&[
            sid.clone(),
            RuntimeValue::Atom("y".into()),
            RuntimeValue::Int(2),
        ])
        .unwrap();
        assert_eq!(host_store_size(&[sid]).unwrap(), RuntimeValue::Int(2));
    }

    #[test]
    fn store_to_list() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);

        host_store_put(&[
            sid.clone(),
            RuntimeValue::String("key".into()),
            RuntimeValue::String("val".into()),
        ])
        .unwrap();

        let list = host_store_to_list(&[sid]).unwrap();
        assert_eq!(
            list,
            RuntimeValue::List(vec![RuntimeValue::Tuple(
                Box::new(RuntimeValue::String("key".into())),
                Box::new(RuntimeValue::String("val".into())),
            )])
        );
    }

    #[test]
    fn store_clear_empties() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id);

        host_store_put(&[
            sid.clone(),
            RuntimeValue::Atom("a".into()),
            RuntimeValue::Int(1),
        ])
        .unwrap();
        host_store_clear(std::slice::from_ref(&sid)).unwrap();

        assert_eq!(host_store_size(&[sid]).unwrap(), RuntimeValue::Int(0));
    }

    #[test]
    fn store_drop_destroys() {
        let _lock = TEST_LOCK.lock().unwrap();
        let id = new_store();
        let sid = RuntimeValue::String(id.clone());

        host_store_drop(std::slice::from_ref(&sid)).unwrap();

        // Operations on dropped store should fail
        let result = host_store_get(&[sid, RuntimeValue::Atom("x".into())]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn store_invalid_id_errors() {
        let _lock = TEST_LOCK.lock().unwrap();
        let bad_id = RuntimeValue::String("nonexistent".into());
        let result = host_store_get(&[bad_id, RuntimeValue::Atom("x".into())]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn store_non_string_id_errors() {
        let _lock = TEST_LOCK.lock().unwrap();
        let result = host_store_get(&[RuntimeValue::Int(42), RuntimeValue::Atom("x".into())]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("store id"));
    }

    #[test]
    fn store_functions_registered() {
        let _lock = TEST_LOCK.lock().unwrap();
        let registry = HostRegistry::new();
        register_store_host_functions(&registry);
        let result = registry.call("store_new", &[]);
        assert!(result.is_ok());
    }
}
