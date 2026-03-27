use rand::Rng;

use super::system::expect_exact_args;
use super::{host_value_kind, HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn host_random_integer(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Random.integer", args, 2)?;
    let min = match &args[0] {
        RuntimeValue::Int(n) => *n,
        other => {
            return Err(HostError::new(format!(
                "Random.integer expects integer arguments, found {}",
                host_value_kind(other)
            )));
        }
    };
    let max = match &args[1] {
        RuntimeValue::Int(n) => *n,
        other => {
            return Err(HostError::new(format!(
                "Random.integer expects integer arguments, found {}",
                host_value_kind(other)
            )));
        }
    };
    if min > max {
        return Err(HostError::new(format!(
            "Random.integer: min ({}) must be <= max ({})",
            min, max
        )));
    }
    let value = rand::rng().random_range(min..=max);
    Ok(RuntimeValue::Int(value))
}

fn host_random_float(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Random.float", args, 0)?;
    let value: f64 = rand::rng().random();
    Ok(RuntimeValue::Float(format!("{}", value)))
}

fn host_random_boolean(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Random.boolean", args, 0)?;
    let value: bool = rand::rng().random();
    Ok(RuntimeValue::Bool(value))
}

pub fn register_random_host_functions(registry: &HostRegistry) {
    registry.register("random_integer", host_random_integer);
    registry.register("random_float", host_random_float);
    registry.register("random_boolean", host_random_boolean);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_integer_in_range() {
        for _ in 0..100 {
            let result =
                host_random_integer(&[RuntimeValue::Int(1), RuntimeValue::Int(10)]).unwrap();
            match result {
                RuntimeValue::Int(n) => assert!((1..=10).contains(&n), "got {}", n),
                other => panic!("expected Int, got {:?}", other),
            }
        }
    }

    #[test]
    fn random_integer_single_value() {
        let result = host_random_integer(&[RuntimeValue::Int(42), RuntimeValue::Int(42)]).unwrap();
        assert_eq!(result, RuntimeValue::Int(42));
    }

    #[test]
    fn random_integer_negative_range() {
        for _ in 0..50 {
            let result =
                host_random_integer(&[RuntimeValue::Int(-5), RuntimeValue::Int(-1)]).unwrap();
            match result {
                RuntimeValue::Int(n) => assert!((-5..=-1).contains(&n), "got {}", n),
                other => panic!("expected Int, got {:?}", other),
            }
        }
    }

    #[test]
    fn random_integer_rejects_min_greater_than_max() {
        let result = host_random_integer(&[RuntimeValue::Int(10), RuntimeValue::Int(1)]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("min (10) must be <= max (1)"));
    }

    #[test]
    fn random_integer_rejects_non_int() {
        let result =
            host_random_integer(&[RuntimeValue::Float("1.5".into()), RuntimeValue::Int(10)]);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("float"));
    }

    #[test]
    fn random_integer_rejects_wrong_arity() {
        let result = host_random_integer(&[RuntimeValue::Int(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn random_float_in_range() {
        for _ in 0..100 {
            let result = host_random_float(&[]).unwrap();
            match result {
                RuntimeValue::Float(s) => {
                    let f: f64 = s.parse().unwrap();
                    assert!((0.0..1.0).contains(&f), "got {}", f);
                }
                other => panic!("expected Float, got {:?}", other),
            }
        }
    }

    #[test]
    fn random_float_rejects_args() {
        let result = host_random_float(&[RuntimeValue::Int(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn random_boolean_returns_bool() {
        for _ in 0..50 {
            let result = host_random_boolean(&[]).unwrap();
            assert!(matches!(result, RuntimeValue::Bool(_)));
        }
    }

    #[test]
    fn random_boolean_rejects_args() {
        let result = host_random_boolean(&[RuntimeValue::Int(1)]);
        assert!(result.is_err());
    }

    #[test]
    fn random_functions_registered() {
        let registry = HostRegistry::new();
        register_random_host_functions(&registry);
        // Verify calls succeed (no get method, so test via call)
        let result = registry.call(
            "random_integer",
            &[RuntimeValue::Int(1), RuntimeValue::Int(10)],
        );
        assert!(result.is_ok());
    }
}
