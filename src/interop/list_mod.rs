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
        other => Err(HostError::new(format!(
            "{} expects list argument {}; found {}",
            function,
            index + 1,
            host_value_kind(other)
        ))),
    }
}

fn host_list_first(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("List.first", args, 1)?;
    let list = expect_list_arg("List.first", args, 0)?;
    Ok(list.into_iter().next().unwrap_or(RuntimeValue::Nil))
}

fn host_list_last(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("List.last", args, 1)?;
    let list = expect_list_arg("List.last", args, 0)?;
    Ok(list.into_iter().last().unwrap_or(RuntimeValue::Nil))
}

fn flatten_value(value: RuntimeValue) -> Vec<RuntimeValue> {
    match value {
        RuntimeValue::List(items) => items.into_iter().flat_map(flatten_value).collect(),
        other => vec![other],
    }
}

fn host_list_flatten(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("List.flatten", args, 1)?;
    let list = expect_list_arg("List.flatten", args, 0)?;
    let flat: Vec<RuntimeValue> = list.into_iter().flat_map(flatten_value).collect();
    Ok(RuntimeValue::List(flat))
}

fn host_list_zip(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("List.zip", args, 2)?;
    let a = expect_list_arg("List.zip", args, 0)?;
    let b = expect_list_arg("List.zip", args, 1)?;
    let zipped: Vec<RuntimeValue> = a
        .into_iter()
        .zip(b)
        .map(|(x, y)| RuntimeValue::Tuple(Box::new(x), Box::new(y)))
        .collect();
    Ok(RuntimeValue::List(zipped))
}

fn host_list_unzip(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("List.unzip", args, 1)?;
    let list = expect_list_arg("List.unzip", args, 0)?;
    let mut lefts = Vec::with_capacity(list.len());
    let mut rights = Vec::with_capacity(list.len());

    for (index, item) in list.into_iter().enumerate() {
        match item {
            RuntimeValue::Tuple(left, right) => {
                lefts.push(*left);
                rights.push(*right);
            }
            other => {
                return Err(HostError::new(format!(
                    "List.unzip entry {} must be a tuple; found {}",
                    index + 1,
                    host_value_kind(&other)
                )));
            }
        }
    }

    Ok(RuntimeValue::Tuple(
        Box::new(RuntimeValue::List(lefts)),
        Box::new(RuntimeValue::List(rights)),
    ))
}

fn host_list_wrap(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("List.wrap", args, 1)?;
    let value = args[0].clone();
    match value {
        RuntimeValue::List(_) => Ok(value),
        RuntimeValue::Nil => Ok(RuntimeValue::List(vec![])),
        other => Ok(RuntimeValue::List(vec![other])),
    }
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn i(n: i64) -> RuntimeValue {
        RuntimeValue::Int(n)
    }

    fn list(items: Vec<RuntimeValue>) -> RuntimeValue {
        RuntimeValue::List(items)
    }

    fn tuple(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
        RuntimeValue::Tuple(Box::new(a), Box::new(b))
    }

    #[test]
    fn list_first_returns_first_element() {
        let result = HOST_REGISTRY
            .call("list_first", &[list(vec![i(1), i(2), i(3)])])
            .expect("list_first should succeed");
        assert_eq!(result, i(1));
    }

    #[test]
    fn list_first_returns_nil_for_empty() {
        let result = HOST_REGISTRY
            .call("list_first", &[list(vec![])])
            .expect("list_first should succeed for empty list");
        assert_eq!(result, RuntimeValue::Nil);
    }

    #[test]
    fn list_last_returns_last_element() {
        let result = HOST_REGISTRY
            .call("list_last", &[list(vec![i(1), i(2), i(3)])])
            .expect("list_last should succeed");
        assert_eq!(result, i(3));
    }

    #[test]
    fn list_flatten_flattens_nested_lists() {
        let nested = list(vec![
            list(vec![i(1), i(2)]),
            i(3),
            list(vec![i(4), list(vec![i(5)])]),
        ]);
        let result = HOST_REGISTRY
            .call("list_flatten", &[nested])
            .expect("list_flatten should succeed");
        assert_eq!(result, list(vec![i(1), i(2), i(3), i(4), i(5)]));
    }

    #[test]
    fn list_zip_zips_two_lists() {
        let result = HOST_REGISTRY
            .call(
                "list_zip",
                &[list(vec![i(1), i(2)]), list(vec![i(3), i(4)])],
            )
            .expect("list_zip should succeed");
        assert_eq!(
            result,
            list(vec![tuple(i(1), i(3)), tuple(i(2), i(4))])
        );
    }

    #[test]
    fn list_unzip_unzips_tuple_list() {
        let zipped = list(vec![tuple(i(1), i(3)), tuple(i(2), i(4))]);
        let result = HOST_REGISTRY
            .call("list_unzip", &[zipped])
            .expect("list_unzip should succeed");
        assert_eq!(
            result,
            tuple(list(vec![i(1), i(2)]), list(vec![i(3), i(4)]))
        );
    }

    #[test]
    fn list_wrap_wraps_non_list_value() {
        let result = HOST_REGISTRY
            .call("list_wrap", &[i(42)])
            .expect("list_wrap should succeed");
        assert_eq!(result, list(vec![i(42)]));
    }

    #[test]
    fn list_wrap_returns_list_unchanged() {
        let input = list(vec![i(1), i(2)]);
        let result = HOST_REGISTRY
            .call("list_wrap", &[input.clone()])
            .expect("list_wrap should succeed for list");
        assert_eq!(result, input);
    }

    #[test]
    fn list_wrap_returns_empty_list_for_nil() {
        let result = HOST_REGISTRY
            .call("list_wrap", &[RuntimeValue::Nil])
            .expect("list_wrap should succeed for nil");
        assert_eq!(result, list(vec![]));
    }
}

pub fn register_list_host_functions(registry: &HostRegistry) {
    registry.register("list_first", host_list_first);
    registry.register("list_last", host_list_last);
    registry.register("list_flatten", host_list_flatten);
    registry.register("list_zip", host_list_zip);
    registry.register("list_unzip", host_list_unzip);
    registry.register("list_wrap", host_list_wrap);
}
