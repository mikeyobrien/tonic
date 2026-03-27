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

pub fn register_assert_host_functions(registry: &HostRegistry) {
    registry.register("assert", host_assert);
    registry.register("refute", host_refute);
    registry.register("assert_equal", host_assert_equal);
    registry.register("assert_not_equal", host_assert_not_equal);
}
