//! Host interop module for Tonic
//!
//! Provides a static extension registry for calling Rust host functions from Tonic code.
//! v1 uses a static registry model (no dynamic plugin loading).

use crate::runtime::RuntimeValue;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{LazyLock, Mutex};

mod enum_mod;
mod float_mod;
mod http_server;
mod integer_mod;
mod io_mod;
mod map_mod;
mod path_mod;
mod string_mod;
mod system;
mod tuple_mod;

/// Host function signature: takes runtime values, returns result
pub type HostFn = fn(&[RuntimeValue]) -> Result<RuntimeValue, HostError>;

/// Errors that can occur during host function execution
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostError {
    message: String,
}

impl HostError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "host error: {}", self.message)
    }
}

impl std::error::Error for HostError {}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub(crate) struct CapturedHostOutput {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Clone, Copy)]
enum HostOutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct ScopedHostInput {
    stdin: String,
    cursor: usize,
}

impl ScopedHostInput {
    fn new(stdin: impl Into<String>) -> Self {
        Self {
            stdin: stdin.into(),
            cursor: 0,
        }
    }

    fn read_line(&mut self) -> String {
        if self.cursor >= self.stdin.len() {
            return String::new();
        }

        let remaining = &self.stdin[self.cursor..];
        let line_end = remaining
            .find('\n')
            .map(|offset| self.cursor + offset + 1)
            .unwrap_or(self.stdin.len());
        let line = self.stdin[self.cursor..line_end].to_string();
        self.cursor = line_end;
        line
    }

    fn read_to_end(&mut self) -> String {
        if self.cursor >= self.stdin.len() {
            return String::new();
        }

        let remaining = self.stdin[self.cursor..].to_string();
        self.cursor = self.stdin.len();
        remaining
    }
}

thread_local! {
    static HOST_OUTPUT_CAPTURE_STACK: RefCell<Vec<CapturedHostOutput>> = const { RefCell::new(Vec::new()) };
    static HOST_INPUT_CAPTURE_STACK: RefCell<Vec<ScopedHostInput>> = const { RefCell::new(Vec::new()) };
}

pub(crate) fn capture_host_output_with_stdin<T>(
    stdin: Option<&str>,
    f: impl FnOnce() -> T,
) -> (T, CapturedHostOutput) {
    HOST_OUTPUT_CAPTURE_STACK.with(|stack| stack.borrow_mut().push(CapturedHostOutput::default()));

    let stdin_pushed = if let Some(stdin) = stdin {
        HOST_INPUT_CAPTURE_STACK.with(|stack| stack.borrow_mut().push(ScopedHostInput::new(stdin)));
        true
    } else {
        false
    };

    let result = f();

    if stdin_pushed {
        let _ = HOST_INPUT_CAPTURE_STACK.with(|stack| stack.borrow_mut().pop());
    }

    let output = HOST_OUTPUT_CAPTURE_STACK
        .with(|stack| stack.borrow_mut().pop())
        .unwrap_or_default();
    (result, output)
}

fn capture_host_stream(stream: HostOutputStream, text: &str) -> bool {
    HOST_OUTPUT_CAPTURE_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        let Some(output) = stack.last_mut() else {
            return false;
        };

        match stream {
            HostOutputStream::Stdout => output.stdout.push_str(text),
            HostOutputStream::Stderr => output.stderr.push_str(text),
        }

        true
    })
}

fn try_read_scoped_host_input<T>(f: impl FnOnce(&mut ScopedHostInput) -> T) -> Option<T> {
    HOST_INPUT_CAPTURE_STACK.with(|stack| {
        let mut stack = stack.borrow_mut();
        let input = stack.last_mut()?;
        Some(f(input))
    })
}

fn write_host_stream(stream: HostOutputStream, text: &str) -> Result<(), HostError> {
    if capture_host_stream(stream, text) {
        return Ok(());
    }

    match stream {
        HostOutputStream::Stdout => std::io::stdout()
            .lock()
            .write_all(text.as_bytes())
            .map_err(|error| HostError::new(format!("failed to write stdout sink: {error}"))),
        HostOutputStream::Stderr => std::io::stderr()
            .lock()
            .write_all(text.as_bytes())
            .map_err(|error| HostError::new(format!("failed to write stderr sink: {error}"))),
    }
}

pub(super) fn flush_host_stdout() -> std::io::Result<()> {
    if HOST_OUTPUT_CAPTURE_STACK.with(|stack| !stack.borrow().is_empty()) {
        return Ok(());
    }

    std::io::stdout().flush()
}

pub(super) fn read_host_stdin_line() -> std::io::Result<String> {
    if let Some(line) = try_read_scoped_host_input(|input| input.read_line()) {
        return Ok(line);
    }

    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line)
}

pub(super) fn read_host_stdin_to_end() -> std::io::Result<String> {
    if let Some(stdin) = try_read_scoped_host_input(|input| input.read_to_end()) {
        return Ok(stdin);
    }

    let mut buffer = Vec::new();
    std::io::stdin().read_to_end(&mut buffer)?;
    Ok(String::from_utf8_lossy(&buffer).into_owned())
}

pub(super) fn write_host_stdout(text: &str) -> Result<(), HostError> {
    write_host_stream(HostOutputStream::Stdout, text)
}

pub(super) fn write_host_stderr(text: &str) -> Result<(), HostError> {
    write_host_stream(HostOutputStream::Stderr, text)
}

pub(super) fn host_value_kind(value: &RuntimeValue) -> &'static str {
    match value {
        RuntimeValue::Int(_) => "int",
        RuntimeValue::Float(_) => "float",
        RuntimeValue::Bool(_) => "bool",
        RuntimeValue::Nil => "nil",
        RuntimeValue::String(_) => "string",
        RuntimeValue::Atom(_) => "atom",
        RuntimeValue::ResultOk(_) | RuntimeValue::ResultErr(_) => "result",
        RuntimeValue::Tuple(_, _) => "tuple",
        RuntimeValue::Map(_) => "map",
        RuntimeValue::Keyword(_) => "keyword",
        RuntimeValue::List(_) => "list",
        RuntimeValue::Binary(_) => "binary",
        RuntimeValue::Range(_, _) => "range",
        RuntimeValue::SteppedRange(_, _, _) => "stepped_range",
        RuntimeValue::Closure(_) => "function",
    }
}

/// Static registry for host functions
pub struct HostRegistry {
    functions: Mutex<HashMap<String, HostFn>>,
}

impl HostRegistry {
    pub fn new() -> Self {
        let registry = Self {
            functions: Mutex::new(HashMap::new()),
        };
        registry.register_sample_functions();
        registry
    }

    /// Register a host function with an atom key
    pub fn register(&self, key: impl Into<String>, function: HostFn) {
        let mut functions = self.functions.lock().unwrap();
        functions.insert(key.into(), function);
    }

    /// Look up and invoke a host function by atom key
    pub fn call(&self, key: &str, args: &[RuntimeValue]) -> Result<RuntimeValue, HostError> {
        let functions = self.functions.lock().unwrap();
        let function = functions
            .get(key)
            .ok_or_else(|| HostError::new(format!("unknown host function: {key}")))?;
        function(args)
    }

    /// Register sample host functions for testing and tooling interop.
    fn register_sample_functions(&self) {
        // :identity - returns its single argument unchanged
        self.register("identity", |args| {
            if args.len() != 1 {
                return Err(HostError::new(format!(
                    "identity expects exactly 1 argument, found {}",
                    args.len()
                )));
            }

            Ok(args[0].clone())
        });

        // :sum_ints - sums integer arguments with strict validation
        self.register("sum_ints", |args| {
            if args.is_empty() {
                return Err(HostError::new("sum_ints expects at least 1 argument"));
            }

            let mut sum = 0i64;
            for (index, value) in args.iter().enumerate() {
                match value {
                    RuntimeValue::Int(number) => sum += number,
                    other => {
                        return Err(HostError::new(format!(
                            "sum_ints expects int arguments only; argument {} was {}",
                            index + 1,
                            host_value_kind(other)
                        )));
                    }
                }
            }

            Ok(RuntimeValue::Int(sum))
        });

        // :make_error - always returns an error
        self.register("make_error", |args| {
            let message = args
                .first()
                .map(|v| v.render())
                .unwrap_or_else(|| "unknown error".to_string());
            Err(HostError::new(message))
        });

        // System interop primitives for tonicctl and similar tooling.
        system::register_system_host_functions(self);

        // String stdlib interop primitives for interpreter-backed String.* calls.
        string_mod::register_string_host_functions(self);

        // Path stdlib interop primitives for interpreter-backed Path.* calls.
        path_mod::register_path_host_functions(self);

        // IO stdlib interop primitives for interpreter-backed IO.* calls.
        io_mod::register_io_host_functions(self);

        // Map stdlib interop primitives for interpreter-backed Map.* calls.
        map_mod::register_map_host_functions(self);

        // Enum stdlib interop primitives for the remaining host-backed Enum.* calls.
        enum_mod::register_enum_host_functions(self);

        // Integer stdlib interop primitives for interpreter-backed Integer.* calls.
        integer_mod::register_integer_host_functions(self);

        // Float stdlib interop primitives for interpreter-backed Float.* calls.
        float_mod::register_float_host_functions(self);

        // Tuple stdlib interop primitives for interpreter-backed Tuple.* and List.to_tuple calls.
        tuple_mod::register_tuple_host_functions(self);

        // HTTP server primitives for tonic-only server code.
        http_server::register_http_server_host_functions(self);
    }
}

impl Default for HostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Global host registry instance
pub static HOST_REGISTRY: LazyLock<HostRegistry> = LazyLock::new(HostRegistry::new);

#[cfg(test)]
static SYSTEM_LOG_ENV_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

#[cfg(test)]
fn map_lookup<'a>(map: &'a RuntimeValue, key: &str) -> Option<&'a RuntimeValue> {
    let RuntimeValue::Map(entries) = map else {
        return None;
    };

    entries.iter().find_map(|(entry_key, entry_value)| {
        if matches!(entry_key, RuntimeValue::Atom(atom) if atom == key) {
            Some(entry_value)
        } else {
            None
        }
    })
}

#[cfg(test)]
#[path = "interop_tests.rs"]
mod tests;
#[cfg(test)]
#[path = "interop_tests_crypto.rs"]
mod tests_crypto;
#[cfg(test)]
#[path = "interop_tests_security.rs"]
mod tests_security;
#[cfg(test)]
#[path = "interop_tests_system.rs"]
mod tests_system;
