use super::system::expect_exact_args;
use super::{HostError, HostRegistry};
use crate::runtime::RuntimeValue;

fn host_integer_to_string(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.to_string", args, 1)?;
    match &args[0] {
        RuntimeValue::Int(n) => Ok(RuntimeValue::String(n.to_string())),
        other => Err(HostError::new(format!(
            "Integer.to_string expects integer argument; found {}",
            super::host_value_kind(other)
        ))),
    }
}

fn host_integer_parse(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("Integer.parse", args, 1)?;
    match &args[0] {
        RuntimeValue::String(s) => {
            // Parse leading integer from string, like Elixir's Integer.parse/1
            let trimmed = s.trim_start();
            if trimmed.is_empty() {
                return Ok(RuntimeValue::Atom("error".to_string()));
            }
            let mut end = 0;
            for (i, ch) in trimmed.char_indices() {
                if i == 0 && (ch == '-' || ch == '+') {
                    end = ch.len_utf8();
                    continue;
                }
                if ch.is_ascii_digit() {
                    end = i + ch.len_utf8();
                } else {
                    break;
                }
            }
            // Must have at least one digit
            let num_part = &trimmed[..end];
            if num_part.is_empty() || num_part == "-" || num_part == "+" {
                return Ok(RuntimeValue::Atom("error".to_string()));
            }
            match num_part.parse::<i64>() {
                Ok(n) => {
                    let rest = trimmed[end..].to_string();
                    Ok(RuntimeValue::Tuple(
                        Box::new(RuntimeValue::Int(n)),
                        Box::new(RuntimeValue::String(rest)),
                    ))
                }
                Err(_) => Ok(RuntimeValue::Atom("error".to_string())),
            }
        }
        other => Err(HostError::new(format!(
            "Integer.parse expects string argument; found {}",
            super::host_value_kind(other)
        ))),
    }
}

pub fn register_integer_host_functions(registry: &HostRegistry) {
    registry.register("integer_to_string", host_integer_to_string);
    registry.register("integer_parse", host_integer_parse);
}
