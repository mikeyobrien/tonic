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

fn host_enum_slice(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.slice", args, 3)?;
    let list = expect_list_arg("Enum.slice", args, 0)?;
    let start = match &args[1] {
        RuntimeValue::Int(n) => *n as usize,
        other => {
            return Err(HostError::new(format!(
                "Enum.slice expects int start; found {}",
                host_value_kind(other)
            )))
        }
    };
    let count = match &args[2] {
        RuntimeValue::Int(n) => *n as usize,
        other => {
            return Err(HostError::new(format!(
                "Enum.slice expects int count; found {}",
                host_value_kind(other)
            )))
        }
    };
    let sliced: Vec<RuntimeValue> = list.into_iter().skip(start).take(count).collect();
    Ok(RuntimeValue::List(sliced))
}

fn host_enum_random(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.random", args, 1)?;
    let list = expect_list_arg("Enum.random", args, 0)?;
    if list.is_empty() {
        return Err(HostError::new("Enum.random called on empty list"));
    }
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as usize;
    let index = nanos % list.len();
    Ok(list[index].clone())
}

fn host_enum_shuffle(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Enum.shuffle", args, 1)?;
    let mut list = expect_list_arg("Enum.shuffle", args, 0)?;
    if list.len() <= 1 {
        return Ok(RuntimeValue::List(list));
    }
    // Fisher-Yates shuffle using system time nanos as randomness source
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos() as u64;
    for i in (1..list.len()).rev() {
        // Simple LCG for pseudo-random numbers
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let j = (seed >> 33) as usize % (i + 1);
        list.swap(i, j);
    }
    Ok(RuntimeValue::List(list))
}

pub fn register_enum_host_functions(registry: &HostRegistry) {
    registry.register("enum_join", host_enum_join);
    registry.register("enum_sort", host_enum_sort);
    registry.register("enum_slice", host_enum_slice);
    registry.register("enum_random", host_enum_random);
    registry.register("enum_shuffle", host_enum_shuffle);
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

    #[test]
    fn enum_join_joins_with_separator() {
        let result = HOST_REGISTRY
            .call("enum_join", &[list(vec![s("a"), s("b"), s("c")]), s(",")])
            .expect("enum_join should succeed");
        assert_eq!(result, s("a,b,c"));
    }

    #[test]
    fn enum_join_renders_non_string_entries() {
        let result = HOST_REGISTRY
            .call(
                "enum_join",
                &[
                    list(vec![s("a"), i(2), RuntimeValue::Atom("ok".to_string())]),
                    s("|"),
                ],
            )
            .expect("enum_join should render non-string entries");
        assert_eq!(result, s("a|2|:ok"));
    }

    #[test]
    fn enum_sort_sorts_integers() {
        let result = HOST_REGISTRY
            .call("enum_sort", &[list(vec![i(3), i(1), i(2)])])
            .expect("enum_sort should succeed");
        assert_eq!(result, list(vec![i(1), i(2), i(3)]));
    }

    #[test]
    fn enum_sort_accepts_ranges() {
        let result = HOST_REGISTRY
            .call("enum_sort", &[RuntimeValue::Range(1, 4)])
            .expect("enum_sort should succeed for ranges");
        assert_eq!(result, list(vec![i(1), i(2), i(3)]));
    }
}
