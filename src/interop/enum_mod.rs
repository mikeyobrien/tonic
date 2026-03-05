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
        RuntimeValue::Range(start, end) => {
            let items = (*start..*end).map(RuntimeValue::Int).collect();
            Ok(items)
        }
        RuntimeValue::SteppedRange(start, end, step) => {
            let mut items = Vec::new();
            let mut current = *start;
            if *step > 0 {
                while current < *end {
                    items.push(RuntimeValue::Int(current));
                    current += step;
                }
            }
            Ok(items)
        }
        other => Err(HostError::new(format!(
            "{} expects list argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_int_arg(function: &str, args: &[RuntimeValue], index: usize) -> Result<i64, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::Int(n) => Ok(*n),
        other => Err(HostError::new(format!(
            "{} expects int argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn expect_string_arg(
    function: &str,
    args: &[RuntimeValue],
    index: usize,
) -> Result<String, HostError> {
    let Some(value) = args.get(index) else {
        return Err(HostError::new(format!(
            "{} missing required argument {}",
            function,
            index + 1
        )));
    };

    match value {
        RuntimeValue::String(s) => Ok(s.clone()),
        other => Err(HostError::new(format!(
            "{} expects string argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn host_enum_count(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.count", args, 1)?;
    let list = expect_list_arg("Enum.count", args, 0)?;
    Ok(RuntimeValue::Int(list.len() as i64))
}

fn host_enum_sum(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.sum", args, 1)?;
    let list = expect_list_arg("Enum.sum", args, 0)?;
    let mut sum = 0i64;
    for (i, item) in list.iter().enumerate() {
        match item {
            RuntimeValue::Int(n) => sum += n,
            other => {
                return Err(HostError::new(format!(
                    "Enum.sum entry {} must be int; found {}",
                    i + 1,
                    host_value_kind(other)
                )));
            }
        }
    }
    Ok(RuntimeValue::Int(sum))
}

fn host_enum_join(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.join", args, 2)?;
    let list = expect_list_arg("Enum.join", args, 0)?;
    let sep = expect_string_arg("Enum.join", args, 1)?;
    let parts: Vec<String> = list
        .into_iter()
        .map(|item| match item {
            RuntimeValue::String(s) => s,
            other => other.render(),
        })
        .collect();
    Ok(RuntimeValue::String(parts.join(&sep)))
}

fn host_enum_sort(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.sort", args, 1)?;
    let mut list = expect_list_arg("Enum.sort", args, 0)?;
    list.sort_by(compare_values);
    Ok(RuntimeValue::List(list))
}

fn compare_values(a: &RuntimeValue, b: &RuntimeValue) -> std::cmp::Ordering {
    match (a, b) {
        (RuntimeValue::Int(x), RuntimeValue::Int(y)) => x.cmp(y),
        (RuntimeValue::Float(x), RuntimeValue::Float(y)) => {
            let xf: f64 = x.parse().unwrap_or(0.0);
            let yf: f64 = y.parse().unwrap_or(0.0);
            xf.partial_cmp(&yf).unwrap_or(std::cmp::Ordering::Equal)
        }
        (RuntimeValue::String(x), RuntimeValue::String(y)) => x.cmp(y),
        (RuntimeValue::Bool(x), RuntimeValue::Bool(y)) => x.cmp(y),
        _ => std::cmp::Ordering::Equal,
    }
}

fn host_enum_reverse(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.reverse", args, 1)?;
    let mut list = expect_list_arg("Enum.reverse", args, 0)?;
    list.reverse();
    Ok(RuntimeValue::List(list))
}

fn host_enum_take(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.take", args, 2)?;
    let list = expect_list_arg("Enum.take", args, 0)?;
    let n = expect_int_arg("Enum.take", args, 1)?;
    let n = n.max(0) as usize;
    Ok(RuntimeValue::List(list.into_iter().take(n).collect()))
}

fn host_enum_drop(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.drop", args, 2)?;
    let list = expect_list_arg("Enum.drop", args, 0)?;
    let n = expect_int_arg("Enum.drop", args, 1)?;
    let n = n.max(0) as usize;
    Ok(RuntimeValue::List(list.into_iter().skip(n).collect()))
}

fn host_enum_chunk_every(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.chunk_every", args, 2)?;
    let list = expect_list_arg("Enum.chunk_every", args, 0)?;
    let n = expect_int_arg("Enum.chunk_every", args, 1)?;

    if n <= 0 {
        return Err(HostError::new(
            "Enum.chunk_every chunk size must be positive",
        ));
    }

    let n = n as usize;
    let chunks: Vec<RuntimeValue> = list
        .chunks(n)
        .map(|chunk| RuntimeValue::List(chunk.to_vec()))
        .collect();
    Ok(RuntimeValue::List(chunks))
}

fn host_enum_unique(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.unique", args, 1)?;
    let list = expect_list_arg("Enum.unique", args, 0)?;
    let mut seen: Vec<RuntimeValue> = Vec::new();
    let unique: Vec<RuntimeValue> = list
        .into_iter()
        .filter(|item| {
            if seen.contains(item) {
                false
            } else {
                seen.push(item.clone());
                true
            }
        })
        .collect();
    Ok(RuntimeValue::List(unique))
}

fn host_enum_into(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.into", args, 2)?;
    let list = expect_list_arg("Enum.into", args, 0)?;
    let collectable = args[1].clone();

    match collectable {
        RuntimeValue::List(mut base) => {
            base.extend(list);
            Ok(RuntimeValue::List(base))
        }
        RuntimeValue::Map(mut base) => {
            for (i, item) in list.into_iter().enumerate() {
                match item {
                    RuntimeValue::Tuple(k, v) => {
                        if let Some(existing) = base.iter_mut().find(|(key, _)| key == k.as_ref())
                        {
                            existing.1 = *v;
                        } else {
                            base.push((*k, *v));
                        }
                    }
                    other => {
                        return Err(HostError::new(format!(
                            "Enum.into entry {} must be a tuple when collecting into map; found {}",
                            i + 1,
                            host_value_kind(&other)
                        )));
                    }
                }
            }
            Ok(RuntimeValue::Map(base))
        }
        other => Err(HostError::new(format!(
            "Enum.into collectable must be list or map; found {}",
            host_value_kind(&other)
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn i(n: i64) -> RuntimeValue {
        RuntimeValue::Int(n)
    }

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    fn list(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::List(items)
    }

    fn atom(a: &str) -> RuntimeValue {
        RuntimeValue::Atom(a.to_string())
    }

    #[test]
    fn enum_count_returns_length() {
        let result = HOST_REGISTRY
            .call("enum_count", &[list(vec![i(1), i(2), i(3)])])
            .expect("enum_count should succeed");
        assert_eq!(result, i(3));
    }

    #[test]
    fn enum_sum_sums_integers() {
        let result = HOST_REGISTRY
            .call("enum_sum", &[list(vec![i(1), i(2), i(3)])])
            .expect("enum_sum should succeed");
        assert_eq!(result, i(6));
    }

    #[test]
    fn enum_sum_rejects_non_integer_entries() {
        let error = HOST_REGISTRY
            .call("enum_sum", &[list(vec![i(1), s("oops")])])
            .expect_err("enum_sum should reject non-integer entries");
        assert!(error.to_string().contains("Enum.sum"));
    }

    #[test]
    fn enum_join_joins_with_separator() {
        let result = HOST_REGISTRY
            .call("enum_join", &[list(vec![s("a"), s("b"), s("c")]), s(",")])
            .expect("enum_join should succeed");
        assert_eq!(result, s("a,b,c"));
    }

    #[test]
    fn enum_sort_sorts_integers() {
        let result = HOST_REGISTRY
            .call("enum_sort", &[list(vec![i(3), i(1), i(2)])])
            .expect("enum_sort should succeed");
        assert_eq!(result, list(vec![i(1), i(2), i(3)]));
    }

    #[test]
    fn enum_reverse_reverses_list() {
        let result = HOST_REGISTRY
            .call("enum_reverse", &[list(vec![i(1), i(2), i(3)])])
            .expect("enum_reverse should succeed");
        assert_eq!(result, list(vec![i(3), i(2), i(1)]));
    }

    #[test]
    fn enum_take_takes_n_elements() {
        let result = HOST_REGISTRY
            .call("enum_take", &[list(vec![i(1), i(2), i(3), i(4)]), i(2)])
            .expect("enum_take should succeed");
        assert_eq!(result, list(vec![i(1), i(2)]));
    }

    #[test]
    fn enum_drop_drops_n_elements() {
        let result = HOST_REGISTRY
            .call("enum_drop", &[list(vec![i(1), i(2), i(3), i(4)]), i(2)])
            .expect("enum_drop should succeed");
        assert_eq!(result, list(vec![i(3), i(4)]));
    }

    #[test]
    fn enum_chunk_every_chunks_list() {
        let result = HOST_REGISTRY
            .call(
                "enum_chunk_every",
                &[list(vec![i(1), i(2), i(3), i(4), i(5)]), i(2)],
            )
            .expect("enum_chunk_every should succeed");
        assert_eq!(
            result,
            list(vec![
                list(vec![i(1), i(2)]),
                list(vec![i(3), i(4)]),
                list(vec![i(5)]),
            ])
        );
    }

    #[test]
    fn enum_chunk_every_rejects_zero_chunk_size() {
        let error = HOST_REGISTRY
            .call("enum_chunk_every", &[list(vec![i(1)]), i(0)])
            .expect_err("enum_chunk_every should reject zero chunk size");
        assert!(error.to_string().contains("Enum.chunk_every"));
    }

    #[test]
    fn enum_unique_removes_duplicates() {
        let result = HOST_REGISTRY
            .call(
                "enum_unique",
                &[list(vec![i(1), i(2), i(1), i(3), i(2)])],
            )
            .expect("enum_unique should succeed");
        assert_eq!(result, list(vec![i(1), i(2), i(3)]));
    }

    #[test]
    fn enum_into_extends_list() {
        let base = list(vec![i(1), i(2)]);
        let additional = list(vec![i(3), i(4)]);
        let result = HOST_REGISTRY
            .call("enum_into", &[additional, base])
            .expect("enum_into should succeed");
        assert_eq!(result, list(vec![i(1), i(2), i(3), i(4)]));
    }

    #[test]
    fn enum_into_collects_tuples_into_map() {
        let pairs = list(vec![
            RuntimeValue::Tuple(Box::new(atom("a")), Box::new(i(1))),
            RuntimeValue::Tuple(Box::new(atom("b")), Box::new(i(2))),
        ]);
        let base = RuntimeValue::Map(vec![]);
        let result = HOST_REGISTRY
            .call("enum_into", &[pairs, base])
            .expect("enum_into should succeed collecting into map");
        assert_eq!(
            result,
            RuntimeValue::Map(vec![(atom("a"), i(1)), (atom("b"), i(2))])
        );
    }

    #[test]
    fn enum_count_works_on_range() {
        let result = HOST_REGISTRY
            .call("enum_count", &[RuntimeValue::Range(1, 6)])
            .expect("enum_count should work on ranges");
        assert_eq!(result, i(5));
    }
}

pub fn register_enum_host_functions(registry: &HostRegistry) {
    registry.register("enum_count", host_enum_count);
    registry.register("enum_sum", host_enum_sum);
    registry.register("enum_join", host_enum_join);
    registry.register("enum_sort", host_enum_sort);
    registry.register("enum_reverse", host_enum_reverse);
    registry.register("enum_take", host_enum_take);
    registry.register("enum_drop", host_enum_drop);
    registry.register("enum_chunk_every", host_enum_chunk_every);
    registry.register("enum_unique", host_enum_unique);
    registry.register("enum_into", host_enum_into);
}
