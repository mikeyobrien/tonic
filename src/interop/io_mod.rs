use super::{
    flush_host_stdout, host_value_kind, read_host_stdin_line, write_host_stderr, write_host_stdout,
    HostError, HostRegistry,
};
use crate::runtime::RuntimeValue;
use termimad::MadSkin;

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

fn value_to_string(value: &RuntimeValue) -> String {
    match value {
        RuntimeValue::String(s) => s.clone(),
        RuntimeValue::Int(i) => i.to_string(),
        RuntimeValue::Float(f) => f.clone(),
        RuntimeValue::Bool(b) => b.to_string(),
        RuntimeValue::Nil => String::new(),
        RuntimeValue::Atom(a) => a.clone(),
        other => other.render(),
    }
}

fn host_io_puts(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.puts", args, 1)?;
    let s = value_to_string(&args[0]);
    write_host_stdout(&format!("{s}\n"))?;
    Ok(RuntimeValue::Nil)
}

fn host_io_inspect(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    if args.is_empty() {
        return Err(HostError::new(
            "IO.inspect expects at least 1 argument, found 0",
        ));
    }

    let value = args[0].clone();
    write_host_stderr(&format!("{}\n", value.render()))?;
    Ok(value)
}

fn host_io_gets(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.gets", args, 1)?;
    let prompt = expect_string_arg("IO.gets", args, 0)?;

    write_host_stdout(&prompt)?;
    flush_host_stdout()
        .map_err(|error| HostError::new(format!("IO.gets failed to flush stdout: {error}")))?;

    let mut line = read_host_stdin_line()
        .map_err(|error| HostError::new(format!("IO.gets failed to read line: {error}")))?;

    // Strip trailing newline like Elixir does
    if line.ends_with('\n') {
        line.pop();
        if line.ends_with('\r') {
            line.pop();
        }
    }

    Ok(RuntimeValue::String(line))
}

fn host_io_render_markdown(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.render_markdown", args, 1)?;
    let markdown = expect_string_arg("IO.render_markdown", args, 0)?;
    let skin = MadSkin::default();
    Ok(RuntimeValue::String(skin.term_text(&markdown).to_string()))
}

fn host_io_ansi_red(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.ansi_red", args, 1)?;
    let s = expect_string_arg("IO.ansi_red", args, 0)?;
    Ok(RuntimeValue::String(format!("\x1b[31m{s}\x1b[0m")))
}

fn host_io_ansi_green(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.ansi_green", args, 1)?;
    let s = expect_string_arg("IO.ansi_green", args, 0)?;
    Ok(RuntimeValue::String(format!("\x1b[32m{s}\x1b[0m")))
}

fn host_io_ansi_yellow(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.ansi_yellow", args, 1)?;
    let s = expect_string_arg("IO.ansi_yellow", args, 0)?;
    Ok(RuntimeValue::String(format!("\x1b[33m{s}\x1b[0m")))
}

fn host_io_ansi_blue(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.ansi_blue", args, 1)?;
    let s = expect_string_arg("IO.ansi_blue", args, 0)?;
    Ok(RuntimeValue::String(format!("\x1b[34m{s}\x1b[0m")))
}

fn host_io_ansi_reset(args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
    expect_exact_args("IO.ansi_reset", args, 0)?;
    Ok(RuntimeValue::String("\x1b[0m".to_string()))
}

pub fn register_io_host_functions(registry: &HostRegistry) {
    registry.register("io_puts", host_io_puts);
    registry.register("io_inspect", host_io_inspect);
    registry.register("io_gets", host_io_gets);
    registry.register("io_render_markdown", host_io_render_markdown);
    registry.register("io_ansi_red", host_io_ansi_red);
    registry.register("io_ansi_green", host_io_ansi_green);
    registry.register("io_ansi_yellow", host_io_ansi_yellow);
    registry.register("io_ansi_blue", host_io_ansi_blue);
    registry.register("io_ansi_reset", host_io_ansi_reset);
}

#[cfg(test)]
mod tests {
    use crate::interop::HOST_REGISTRY;
    use crate::runtime::RuntimeValue;

    fn s(text: &str) -> RuntimeValue {
        RuntimeValue::String(text.to_string())
    }

    #[test]
    fn io_render_markdown_renders_terminal_text() {
        let result = HOST_REGISTRY
            .call("io_render_markdown", &[s("# Title\n\n- item")])
            .expect("io_render_markdown should succeed");
        let RuntimeValue::String(rendered) = result else {
            panic!("io_render_markdown should return a string");
        };
        assert!(rendered.contains("Title"));
        assert!(rendered.contains("item"));
        assert!(!rendered.contains("# Title"));
        assert!(rendered.contains("\x1b["));
    }

    #[test]
    fn io_ansi_red_wraps_in_ansi_codes() {
        let result = HOST_REGISTRY
            .call("io_ansi_red", &[s("hello")])
            .expect("io_ansi_red should succeed");
        assert_eq!(result, s("\x1b[31mhello\x1b[0m"));
    }

    #[test]
    fn io_ansi_green_wraps_in_ansi_codes() {
        let result = HOST_REGISTRY
            .call("io_ansi_green", &[s("hello")])
            .expect("io_ansi_green should succeed");
        assert_eq!(result, s("\x1b[32mhello\x1b[0m"));
    }

    #[test]
    fn io_ansi_yellow_wraps_in_ansi_codes() {
        let result = HOST_REGISTRY
            .call("io_ansi_yellow", &[s("warn")])
            .expect("io_ansi_yellow should succeed");
        assert_eq!(result, s("\x1b[33mwarn\x1b[0m"));
    }

    #[test]
    fn io_ansi_blue_wraps_in_ansi_codes() {
        let result = HOST_REGISTRY
            .call("io_ansi_blue", &[s("info")])
            .expect("io_ansi_blue should succeed");
        assert_eq!(result, s("\x1b[34minfo\x1b[0m"));
    }

    #[test]
    fn io_ansi_reset_returns_reset_sequence() {
        let result = HOST_REGISTRY
            .call("io_ansi_reset", &[])
            .expect("io_ansi_reset should succeed");
        assert_eq!(result, s("\x1b[0m"));
    }

    #[test]
    fn io_render_markdown_rejects_wrong_arity() {
        let error = HOST_REGISTRY
            .call("io_render_markdown", &[])
            .expect_err("io_render_markdown should reject zero arguments");
        assert_eq!(
            error.to_string(),
            "host error: IO.render_markdown expects exactly 1 argument, found 0"
        );
    }

    #[test]
    fn io_ansi_red_rejects_wrong_arity() {
        let error = HOST_REGISTRY
            .call("io_ansi_red", &[])
            .expect_err("io_ansi_red should reject zero arguments");
        assert_eq!(
            error.to_string(),
            "host error: IO.ansi_red expects exactly 1 argument, found 0"
        );
    }

    #[test]
    fn io_puts_accepts_non_string() {
        let result = HOST_REGISTRY
            .call("io_puts", &[RuntimeValue::Int(42)])
            .expect("io_puts should accept non-string");
        assert_eq!(result, RuntimeValue::Nil);
    }

    #[test]
    fn io_inspect_returns_value_unchanged() {
        let value = RuntimeValue::Int(42);
        let result = HOST_REGISTRY
            .call("io_inspect", std::slice::from_ref(&value))
            .expect("io_inspect should succeed");
        assert_eq!(result, value);
    }
}
