use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn extract_message(args: &[RuntimeValue], index: usize, default: &str) -> String {
    args.get(index)
        .and_then(|v| match v {
            RuntimeValue::Nil => None,
            RuntimeValue::String(s) => Some(s.clone()),
            other => Some(other.render()),
        })
        .unwrap_or_else(|| default.to_string())
}

/// Assert that a value is truthy (not false, nil, or err).
fn host_assert(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.is_empty() || args.len() > 2 {
        return Err(HostError::new(
            "Assert.assert expects 1-2 arguments (value, optional message)",
        ));
    }
    let value = &args[0];
    let message = extract_message(args, 1, "assertion failed: expected truthy value");

    match value {
        RuntimeValue::Bool(false) | RuntimeValue::Nil => {
            Ok(RuntimeValue::ResultErr(Box::new(RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("assertion_failed".to_string())),
                Box::new(RuntimeValue::Tuple(
                    Box::new(RuntimeValue::Atom("assert".to_string())),
                    Box::new(RuntimeValue::String(message)),
                )),
            ))))
        }
        _ => Ok(RuntimeValue::Atom("ok".to_string())),
    }
}

/// Assert that a value is falsy (false, nil).
fn host_refute(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.is_empty() || args.len() > 2 {
        return Err(HostError::new(
            "Assert.refute expects 1-2 arguments (value, optional message)",
        ));
    }
    let value = &args[0];
    let message = extract_message(args, 1, "refute failed: expected falsy value");

    match value {
        RuntimeValue::Bool(false) | RuntimeValue::Nil => Ok(RuntimeValue::Atom("ok".to_string())),
        _ => Ok(RuntimeValue::ResultErr(Box::new(RuntimeValue::Tuple(
            Box::new(RuntimeValue::Atom("assertion_failed".to_string())),
            Box::new(RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("refute".to_string())),
                Box::new(RuntimeValue::String(message)),
            )),
        )))),
    }
}

/// Assert that two values are equal.
fn host_assert_equal(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(HostError::new(
            "Assert.assert_equal expects 2-3 arguments (left, right, optional message)",
        ));
    }
    let left = &args[0];
    let right = &args[1];

    if left == right {
        return Ok(RuntimeValue::Atom("ok".to_string()));
    }

    let message = extract_message(args, 2, "values are not equal");

    Ok(RuntimeValue::ResultErr(Box::new(RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("assertion_failed".to_string())),
        Box::new(RuntimeValue::List(vec![
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("type".to_string())),
                Box::new(RuntimeValue::Atom("assert_equal".to_string())),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("left".to_string())),
                Box::new(left.clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("right".to_string())),
                Box::new(right.clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("message".to_string())),
                Box::new(RuntimeValue::String(message)),
            ),
        ])),
    ))))
}

/// Assert that two values are not equal.
fn host_assert_not_equal(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(HostError::new(
            "Assert.assert_not_equal expects 2-3 arguments (left, right, optional message)",
        ));
    }
    let left = &args[0];
    let right = &args[1];

    if left != right {
        return Ok(RuntimeValue::Atom("ok".to_string()));
    }

    let message = extract_message(args, 2, "values should not be equal");

    Ok(RuntimeValue::ResultErr(Box::new(RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("assertion_failed".to_string())),
        Box::new(RuntimeValue::List(vec![
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("type".to_string())),
                Box::new(RuntimeValue::Atom("assert_not_equal".to_string())),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("left".to_string())),
                Box::new(left.clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("right".to_string())),
                Box::new(right.clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("message".to_string())),
                Box::new(RuntimeValue::String(message)),
            ),
        ])),
    ))))
}

/// Assert that a container includes an element.
/// For strings: checks substring containment. For lists: checks membership.
fn host_assert_contains(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.len() < 2 || args.len() > 3 {
        return Err(HostError::new(
            "Assert.assert_contains expects 2-3 arguments (container, element, optional message)",
        ));
    }
    let container = &args[0];
    let element = &args[1];

    let contained = match (container, element) {
        (RuntimeValue::String(haystack), RuntimeValue::String(needle)) => {
            haystack.contains(needle.as_str())
        }
        (RuntimeValue::List(items), _) => items.contains(element),
        _ => {
            return Err(HostError::new(
                "Assert.assert_contains expects a String or List as the first argument",
            ));
        }
    };

    if contained {
        return Ok(RuntimeValue::Atom("ok".to_string()));
    }

    let message = extract_message(args, 2, "element not found in container");

    Ok(RuntimeValue::ResultErr(Box::new(RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("assertion_failed".to_string())),
        Box::new(RuntimeValue::List(vec![
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("type".to_string())),
                Box::new(RuntimeValue::Atom("assert_contains".to_string())),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("container".to_string())),
                Box::new(container.clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("element".to_string())),
                Box::new(element.clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("message".to_string())),
                Box::new(RuntimeValue::String(message)),
            ),
        ])),
    ))))
}

/// Extract a numeric value as f64 from Int or Float variants.
fn extract_f64(val: &RuntimeValue) -> Option<f64> {
    match val {
        RuntimeValue::Int(i) => Some(*i as f64),
        RuntimeValue::Float(s) => s.parse::<f64>().ok(),
        _ => None,
    }
}

/// Assert that two numeric values are within delta of each other.
fn host_assert_in_delta(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.len() < 3 || args.len() > 4 {
        return Err(HostError::new(
            "Assert.assert_in_delta expects 3-4 arguments (left, right, delta, optional message)",
        ));
    }

    let left_f = extract_f64(&args[0]).ok_or_else(|| {
        HostError::new("Assert.assert_in_delta: left must be a number (Int or Float)")
    })?;
    let right_f = extract_f64(&args[1]).ok_or_else(|| {
        HostError::new("Assert.assert_in_delta: right must be a number (Int or Float)")
    })?;
    let delta_f = extract_f64(&args[2]).ok_or_else(|| {
        HostError::new("Assert.assert_in_delta: delta must be a number (Int or Float)")
    })?;

    if (left_f - right_f).abs() <= delta_f {
        return Ok(RuntimeValue::Atom("ok".to_string()));
    }

    let message = extract_message(args, 3, "values are not within delta");

    Ok(RuntimeValue::ResultErr(Box::new(RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("assertion_failed".to_string())),
        Box::new(RuntimeValue::List(vec![
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("type".to_string())),
                Box::new(RuntimeValue::Atom("assert_in_delta".to_string())),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("left".to_string())),
                Box::new(args[0].clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("right".to_string())),
                Box::new(args[1].clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("delta".to_string())),
                Box::new(args[2].clone()),
            ),
            RuntimeValue::Tuple(
                Box::new(RuntimeValue::Atom("message".to_string())),
                Box::new(RuntimeValue::String(message)),
            ),
        ])),
    ))))
}

/// Skip the current test with an optional reason.
fn host_skip(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.len() > 1 {
        return Err(HostError::new(
            "Assert.skip expects 0-1 arguments (optional reason)",
        ));
    }
    let reason = extract_message(args, 0, "");
    Ok(RuntimeValue::ResultErr(Box::new(RuntimeValue::Tuple(
        Box::new(RuntimeValue::Atom("test_skipped".to_string())),
        Box::new(RuntimeValue::String(reason)),
    ))))
}

pub fn register_assert_host_functions(registry: &HostRegistry) {
    registry.register("assert", host_assert);
    registry.register("refute", host_refute);
    registry.register("assert_equal", host_assert_equal);
    registry.register("assert_not_equal", host_assert_not_equal);
    registry.register("assert_contains", host_assert_contains);
    registry.register("assert_in_delta", host_assert_in_delta);
    registry.register("skip", host_skip);
}
