mod acceptance;
mod c_backend;
mod cache;
mod cli_diag;
mod deps;
mod docs;
mod formatter;
mod guard_builtins;
mod interop;
mod ir;
mod lexer;
mod linker;
mod llvm_backend;
#[cfg(feature = "lsp")]
mod lsp;
mod manifest;
mod mir;
pub mod native_abi;
mod native_artifact;
pub mod native_runtime;
mod observability;
mod parser;
mod profiling;
mod repl;
mod resolver;
mod resolver_diag;
mod runtime;
mod stdlib_catalog;
mod target;
mod test_runner;
mod typing;

use acceptance::{load_acceptance_yaml, load_feature_scenarios, BenchmarkMetrics};
use cache::{
    build_run_cache_key, load_cached_ir, should_trace_cache_status, store_cached_ir,
    trace_cache_status,
};
use cli_diag::{CliDiagnostic, EXIT_FAILURE, EXIT_OK};
use observability::{ErrorSource, ObservabilityError, ObservabilityRun};

/// A compilation error that preserves source offset for snippet rendering.
#[derive(Debug)]
struct CompileError {
    kind: &'static str,
    phase: &'static str,
    diagnostic_code: Option<String>,
    message: String,
    offset: Option<usize>,
}

impl CompileError {
    fn new(kind: &'static str, phase: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            phase,
            diagnostic_code: None,
            message: message.into(),
            offset: None,
        }
    }

    fn with_offset(
        kind: &'static str,
        phase: &'static str,
        message: impl Into<String>,
        offset: Option<usize>,
    ) -> Self {
        let message = message.into();
        Self {
            kind,
            phase,
            diagnostic_code: extract_diagnostic_code(&message),
            message,
            offset,
        }
    }

    fn from_lexer(error: lexer::LexerError) -> Self {
        Self::with_offset(
            "lexer_error",
            "frontend.scan_tokens",
            error.to_string(),
            Some(error.offset()),
        )
    }

    fn from_parser(error: parser::ParserError) -> Self {
        Self::with_offset(
            "parser_error",
            "frontend.parse_ast",
            error.to_string(),
            error.offset(),
        )
    }

    fn from_resolver(error: resolver_diag::ResolverError) -> Self {
        Self::with_offset(
            "resolver_error",
            "frontend.resolve_ast",
            error.to_string(),
            error.offset(),
        )
    }

    fn from_typing_message(message: impl Into<String>, offset: Option<usize>) -> Self {
        Self::with_offset("typing_error", "frontend.infer_types", message, offset)
    }

    fn from_ir_lowering(error: impl ToString) -> Self {
        Self::new("ir_lowering_error", "frontend.lower_ir", error.to_string())
    }

    fn into_diagnostic(self, filename: Option<&str>, source: &str) -> CliDiagnostic {
        CliDiagnostic::failure_with_filename_and_source(self.message, filename, source, self.offset)
    }

    fn to_observability_error(&self, filename: &str, source: &str) -> ObservabilityError {
        ObservabilityError {
            kind: self.kind.to_string(),
            diagnostic_code: self.diagnostic_code.clone(),
            phase: Some(self.phase.to_string()),
            message: self.message.clone(),
            source: observability_error_source(filename, source, self.offset),
        }
    }
}
use formatter::{format_path, FormatMode};
use ir::{lower_ast_to_ir, IrProgram};
use lexer::scan_tokens;
use manifest::load_run_source;
use mir::{lower_ir_to_mir, optimize_for_native_backend};
use parser::parse_ast;
use resolver::resolve_ast;
use runtime::{evaluate_entrypoint, RuntimeValue};
use test_runner::{TestOutputFormat, TestRunnerError};
use typing::infer_types;

fn command_argv(command: &str, args: &[String]) -> Vec<String> {
    let mut argv = Vec::with_capacity(args.len() + 1);
    argv.push(command.to_string());
    argv.extend(args.iter().cloned());
    argv
}

fn observe_phase<T>(
    profiler: &mut Option<profiling::PhaseProfiler>,
    observed_run: &mut Option<ObservabilityRun>,
    phase_name: &str,
    run: impl FnOnce() -> T,
) -> T {
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.phase(phase_name, || {
            profiling::profile_phase(profiler, phase_name, run)
        })
    } else {
        profiling::profile_phase(profiler, phase_name, run)
    }
}

fn observe_phase_result<T, E>(
    profiler: &mut Option<profiling::PhaseProfiler>,
    observed_run: &mut Option<ObservabilityRun>,
    phase_name: &str,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.phase_result(phase_name, || {
            profiling::profile_phase(profiler, phase_name, run)
        })
    } else {
        profiling::profile_phase(profiler, phase_name, run)
    }
}

fn observe_command_phase<T>(
    observed_run: &mut Option<ObservabilityRun>,
    phase_name: &str,
    run: impl FnOnce() -> T,
) -> T {
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.phase(phase_name, run)
    } else {
        run()
    }
}

fn observe_command_phase_result<T, E>(
    observed_run: &mut Option<ObservabilityRun>,
    phase_name: &str,
    run: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.phase_result(phase_name, run)
    } else {
        run()
    }
}

fn extract_diagnostic_code(message: &str) -> Option<String> {
    let stripped = message.strip_prefix('[')?;
    let code = stripped.split(']').next()?;
    (!code.is_empty()).then(|| code.to_string())
}

fn observability_error_source(
    path: &str,
    source: &str,
    offset: Option<usize>,
) -> Option<ErrorSource> {
    let offset = offset?;
    if offset > source.len() || !source.is_char_boundary(offset) {
        return None;
    }

    let before = &source[..offset];
    let line_start = before.rfind('\n').map(|index| index + 1).unwrap_or(0);
    let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = source[line_start..offset].chars().count() + 1;

    Some(ErrorSource {
        path: path.to_string(),
        line,
        column,
        offset,
    })
}

fn make_observability_error(
    kind: &'static str,
    phase: &'static str,
    message: impl Into<String>,
    source: Option<ErrorSource>,
) -> ObservabilityError {
    let message = message.into();
    ObservabilityError {
        kind: kind.to_string(),
        diagnostic_code: extract_diagnostic_code(&message),
        phase: Some(phase.to_string()),
        message,
        source,
    }
}

fn emit_observability_warnings(warnings: Vec<String>) {
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
}

fn finalize_observed_run(
    observed_run: &mut Option<ObservabilityRun>,
    exit_code: i32,
    error: Option<ObservabilityError>,
) -> i32 {
    if let Some(observed_run) = observed_run.as_mut() {
        emit_observability_warnings(observed_run.finish_with_status(exit_code, error));
    }
    exit_code
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VerifyMode {
    Auto,
    Mixed,
    Manual,
}

impl VerifyMode {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "auto" => Some(Self::Auto),
            "mixed" => Some(Self::Mixed),
            "manual" => Some(Self::Manual),
            _ => None,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Mixed => "mixed",
            Self::Manual => "manual",
        }
    }

    fn selected_tags(self) -> &'static [&'static str] {
        match self {
            Self::Auto => &["@auto"],
            Self::Mixed => &["@auto", "@agent-manual"],
            Self::Manual => &["@auto", "@agent-manual", "@human-manual"],
        }
    }
}

const BENCHMARK_THRESHOLD_COLD_START_P50_MS: u64 = 50;
const BENCHMARK_THRESHOLD_WARM_START_P50_MS: u64 = 10;
const BENCHMARK_THRESHOLD_IDLE_RSS_MB: u64 = 30;

fn main() {
    // Spawn on a thread with 64MB stack to support deeply recursive Tonic programs.
    // Rust's default 8MB stack overflows on idiomatic recursive code (e.g. brainfuck_interpreter).
    const STACK_SIZE: usize = 64 * 1024 * 1024;
    let builder = std::thread::Builder::new().stack_size(STACK_SIZE);
    let handler = builder
        .spawn(|| run(std::env::args().skip(1).collect()))
        .expect("failed to spawn main thread");
    std::process::exit(handler.join().expect("main thread panicked"));
}

fn run(args: Vec<String>) -> i32 {
    let mut iter = args.into_iter();

    match iter.next().as_deref() {
        None | Some("-h") | Some("--help") => {
            print_help();
            EXIT_OK
        }
        Some("run") => handle_run(iter.collect()),
        Some("check") => handle_check(iter.collect()),
        Some("test") => handle_test(iter.collect()),
        Some("fmt") => handle_fmt(iter.collect()),
        Some("compile") => handle_compile(iter.collect()),
        Some("cache") => run_placeholder("cache"),
        Some("repl") => repl::handle_repl(iter.collect()),
        Some("verify") => handle_verify(iter.collect()),
        Some("deps") => handle_deps(iter.collect()),
        Some("publish") => handle_publish(iter.collect()),
        Some("install") => handle_install(iter.collect()),
        Some("uninstall") => handle_uninstall(iter.collect()),
        Some("installed") => handle_installed(iter.collect()),
        Some("docs") => docs::handle_docs(iter.collect()),
        #[cfg(feature = "lsp")]
        Some("lsp") => {
            lsp::run_lsp_server();
            EXIT_OK
        }
        Some(other) => CliDiagnostic::usage_with_hint(
            format!("unknown command '{other}'"),
            "run `tonic --help` to see available commands",
        )
        .emit(),
    }
}

#[path = "cmd_run.rs"]
mod cmd_run;
use cmd_run::*;

#[path = "cmd_check.rs"]
mod cmd_check;
use cmd_check::*;

#[path = "cmd_test.rs"]
mod cmd_test;
use cmd_test::*;

#[path = "cmd_compile.rs"]
mod cmd_compile;
use cmd_compile::*;

#[path = "cmd_verify.rs"]
mod cmd_verify;
use cmd_verify::*;

#[path = "cmd_deps.rs"]
mod cmd_deps;
use cmd_deps::*;

#[path = "cmd_install.rs"]
mod cmd_install;
use cmd_install::*;
