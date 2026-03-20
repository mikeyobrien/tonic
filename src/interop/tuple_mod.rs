use super::system::expect_exact_args;
use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn host_tuple_to_list(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Tuple.to_list", args, 1)?;
    match &args[0] {
        RuntimeValue::Tuple(a, b) => Ok(RuntimeValue::List(vec![*a.clone(), *b.clone()])),
        other => Err(HostError::new(format!(
            "Tuple.to_list expects tuple argument; found {}",
            super::host_value_kind(other)
        ))),
    }
}

fn host_list_to_tuple(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("List.to_tuple", args, 1)?;
    match &args[0] {
        RuntimeValue::List(items) => {
            if items.len() != 2 {
                return Err(HostError::new(format!(
                    "List.to_tuple expects a 2-element list, found {} elements",
                    items.len()
                )));
            }
            Ok(RuntimeValue::Tuple(
                Box::new(items[0].clone()),
                Box::new(items[1].clone()),
            ))
        }
        other => Err(HostError::new(format!(
            "List.to_tuple expects list argument; found {}",
            super::host_value_kind(other)
        ))),
    }
}

pub fn register_tuple_host_functions(registry: &HostRegistry) {
    registry.register("tuple_to_list", host_tuple_to_list);
    registry.register("list_to_tuple", host_list_to_tuple);
}
