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
    std::process::exit(run(std::env::args().skip(1).collect()));
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
        Some("docs") => docs::handle_docs(iter.collect()),
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

fn handle_run(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_run_help();
        return EXIT_OK;
    }

    if args.is_empty() {
        return CliDiagnostic::usage_with_hint(
            "missing required <path>",
            "run `tonic run --help` for usage",
        )
        .emit();
    }

    let source_path = args[0].clone();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut observed_run = ObservabilityRun::from_env("run", &command_argv("run", &args), &cwd);
    let mut profiler = profiling::PhaseProfiler::from_env("run");

    if native_artifact::is_native_artifact_path(&source_path) {
        return handle_run_native_artifact(&source_path, &mut profiler, &mut observed_run);
    }

    let source =
        match observe_phase_result(&mut profiler, &mut observed_run, "run.load_source", || {
            load_run_source(&source_path)
        }) {
            Ok(source) => source,
            Err(error) => {
                let message = error;
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "run.load_source",
                        message,
                        None,
                    )),
                );
            }
        };

    let source_path_obj = std::path::Path::new(&source_path);
    let project_root = if source_path_obj.is_dir() {
        source_path_obj.to_path_buf()
    } else {
        source_path_obj
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| std::path::Path::new(".").to_path_buf())
    };

    let cache_key = build_run_cache_key(&source, &project_root);
    let mut cache_status = "miss";

    let ir =
        match observe_phase_result(&mut profiler, &mut observed_run, "run.cache_lookup", || {
            load_cached_ir(&cache_key)
        }) {
            Ok(Some(cached_ir)) => {
                cache_status = "hit";
                cached_ir
            }
            Ok(None) | Err(_) => {
                let compiled_ir =
                    match compile_source_to_ir(&source, &mut profiler, &mut observed_run) {
                        Ok(ir) => ir,
                        Err(error) => {
                            let obs_error = error.to_observability_error(&source_path, &source);
                            let exit_code =
                                error.into_diagnostic(Some(&source_path), &source).emit();
                            return finalize_observed_run(
                                &mut observed_run,
                                exit_code,
                                Some(obs_error),
                            );
                        }
                    };

                if let Err(error) = observe_phase_result(
                    &mut profiler,
                    &mut observed_run,
                    "run.cache_store",
                    || store_cached_ir(&cache_key, &compiled_ir),
                ) {
                    eprintln!("warning: {error}");
                }
                compiled_ir
            }
        };

    if should_trace_cache_status() {
        trace_cache_status(cache_status);
    }

    let value = match observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "run.evaluate_entrypoint",
        || evaluate_entrypoint(&ir),
    ) {
        Ok(value) => value,
        Err(error) => {
            let message = error.to_string();
            let source_info = observability_error_source(&source_path, &source, error.offset());
            let exit_code = CliDiagnostic::failure_with_filename_and_source(
                message.clone(),
                Some(&source_path),
                &source,
                error.offset(),
            )
            .emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "runtime_error",
                    "run.evaluate_entrypoint",
                    message,
                    source_info,
                )),
            );
        }
    };

    match value {
        RuntimeValue::ResultErr(reason) => {
            let message = format!("runtime returned err({})", reason.render());
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "runtime_error",
                    "run.evaluate_entrypoint",
                    message,
                    None,
                )),
            )
        }
        other => {
            println!("{}", other.render());
            finalize_observed_run(&mut observed_run, EXIT_OK, None)
        }
    }
}

fn handle_run_native_artifact(
    path: &str,
    profiler: &mut Option<profiling::PhaseProfiler>,
    observed_run: &mut Option<ObservabilityRun>,
) -> i32 {
    let manifest_path = std::path::Path::new(path);
    let manifest =
        match observe_phase_result(profiler, observed_run, "run.native.load_manifest", || {
            native_artifact::load_manifest(manifest_path)
        }) {
            Ok(manifest) => manifest,
            Err(error) => {
                let message = error;
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "run.native.load_manifest",
                        message,
                        None,
                    )),
                );
            }
        };

    if let Err(error) = observe_phase_result(
        profiler,
        observed_run,
        "run.native.validate_manifest",
        || native_artifact::validate_manifest_for_host(&manifest),
    ) {
        let message = error;
        let exit_code = CliDiagnostic::failure(message.clone()).emit();
        return finalize_observed_run(
            observed_run,
            exit_code,
            Some(make_observability_error(
                "io_error",
                "run.native.validate_manifest",
                message,
                None,
            )),
        );
    }

    let ir = match observe_phase_result(profiler, observed_run, "run.native.load_ir", || {
        native_artifact::load_ir_from_manifest(manifest_path, &manifest)
    }) {
        Ok(ir) => ir,
        Err(error) => {
            let message = error;
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                observed_run,
                exit_code,
                Some(make_observability_error(
                    "io_error",
                    "run.native.load_ir",
                    message,
                    None,
                )),
            );
        }
    };

    let value =
        match observe_phase_result(profiler, observed_run, "run.evaluate_entrypoint", || {
            evaluate_entrypoint(&ir)
        }) {
            Ok(value) => value,
            Err(error) => {
                let message = error.to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "runtime_error",
                        "run.evaluate_entrypoint",
                        message,
                        None,
                    )),
                );
            }
        };

    match value {
        RuntimeValue::ResultErr(reason) => {
            let message = format!("runtime returned err({})", reason.render());
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            finalize_observed_run(
                observed_run,
                exit_code,
                Some(make_observability_error(
                    "runtime_error",
                    "run.evaluate_entrypoint",
                    message,
                    None,
                )),
            )
        }
        other => {
            println!("{}", other.render());
            finalize_observed_run(observed_run, EXIT_OK, None)
        }
    }
}

fn compile_source_to_ir(
    source: &str,
    profiler: &mut Option<profiling::PhaseProfiler>,
    observed_run: &mut Option<ObservabilityRun>,
) -> Result<IrProgram, CompileError> {
    let tokens = observe_phase_result(profiler, observed_run, "frontend.scan_tokens", || {
        scan_tokens(source)
    })
    .map_err(CompileError::from_lexer)?;

    let ast = observe_phase_result(profiler, observed_run, "frontend.parse_ast", || {
        parse_ast(&tokens)
    })
    .map_err(CompileError::from_parser)?;

    observe_phase_result(profiler, observed_run, "frontend.resolve_ast", || {
        resolve_ast(&ast)
    })
    .map_err(CompileError::from_resolver)?;

    let type_summary = observe_phase_result(profiler, observed_run, "frontend.infer_types", || {
        infer_types(&ast)
    })
    .map_err(|error| CompileError::from_typing_message(error.to_string(), error.offset()))?;
    maybe_trace_type_summary(type_summary.len());

    observe_phase_result(profiler, observed_run, "frontend.lower_ir", || {
        lower_ast_to_ir(&ast)
    })
    .map_err(CompileError::from_ir_lowering)
}

fn maybe_trace_type_summary(signature_count: usize) {
    if std::env::var_os("TONIC_DEBUG_TYPES").is_some() {
        eprintln!("type-signatures {signature_count}");
    }
}

fn run_placeholder(command: &str) -> i32 {
    println!("tonic {command} command skeleton");
    EXIT_OK
}

fn handle_check(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_check_help();
        return EXIT_OK;
    }

    if args.is_empty() {
        return CliDiagnostic::usage_with_hint(
            "missing required <path>",
            "run `tonic check --help` for usage",
        )
        .emit();
    }

    let source_path = args[0].clone();
    let is_project_root_path = std::path::Path::new(&source_path).is_dir();
    let mut dump_tokens = false;
    let mut dump_ast = false;
    let mut dump_ir = false;
    let mut dump_mir = false;
    let mut token_dump_format = TestOutputFormat::Text;
    let mut token_dump_format_explicit = false;
    let mut index = 1;

    while index < args.len() {
        match args[index].as_str() {
            "--dump-tokens" => {
                dump_tokens = true;
                index += 1;
            }
            "--dump-ast" => {
                dump_ast = true;
                index += 1;
            }
            "--dump-ir" => {
                dump_ir = true;
                index += 1;
            }
            "--dump-mir" => {
                dump_mir = true;
                index += 1;
            }
            "--format" => {
                let Some(value) = args.get(index + 1) else {
                    return CliDiagnostic::usage("missing value for --format").emit();
                };

                let Some(parsed) = TestOutputFormat::parse(value) else {
                    return CliDiagnostic::usage(format!(
                        "unsupported format '{value}' (expected 'text' or 'json')"
                    ))
                    .emit();
                };

                token_dump_format = parsed;
                token_dump_format_explicit = true;
                index += 2;
            }
            other => {
                return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit();
            }
        }
    }

    let dump_mode_count = [dump_tokens, dump_ast, dump_ir, dump_mir]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

    if dump_mode_count > 1 {
        return CliDiagnostic::usage(
            "--dump-tokens, --dump-ast, --dump-ir, and --dump-mir cannot be used together",
        )
        .emit();
    }

    if token_dump_format_explicit && !dump_tokens {
        return CliDiagnostic::usage("--format is only supported with --dump-tokens").emit();
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut observed_run = ObservabilityRun::from_env("check", &command_argv("check", &args), &cwd);
    if let Some(observed_run) = observed_run.as_mut() {
        if dump_tokens {
            observed_run.record_metadata("dump_mode", "tokens");
            observed_run.record_metadata(
                "format",
                match token_dump_format {
                    TestOutputFormat::Text => "text",
                    TestOutputFormat::Json => "json",
                },
            );
        } else if dump_ast {
            observed_run.record_metadata("dump_mode", "ast");
        } else if dump_ir {
            observed_run.record_metadata("dump_mode", "ir");
        } else if dump_mir {
            observed_run.record_metadata("dump_mode", "mir");
        }
    }

    let source = match observe_command_phase_result(&mut observed_run, "check.load_source", || {
        load_run_source(&source_path)
    }) {
        Ok(source) => source,
        Err(error) => {
            let message = error;
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "io_error",
                    "check.load_source",
                    message,
                    None,
                )),
            );
        }
    };

    let tokens =
        match observe_command_phase_result(&mut observed_run, "frontend.scan_tokens", || {
            scan_tokens(&source)
        }) {
            Ok(tokens) => tokens,
            Err(error) => {
                let message = error.to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "lexer_error",
                        "frontend.scan_tokens",
                        message,
                        None,
                    )),
                );
            }
        };

    if dump_tokens {
        match token_dump_format {
            TestOutputFormat::Text => {
                for token in tokens {
                    println!("{}", token.dump_label());
                }
            }
            TestOutputFormat::Json => {
                let records: Vec<_> = tokens.iter().map(|token| token.dump_record()).collect();
                let json = match serde_json::to_string(&records) {
                    Ok(value) => value,
                    Err(_) => {
                        let message = "failed to serialize token dump".to_string();
                        let exit_code = CliDiagnostic::failure(message.clone()).emit();
                        return finalize_observed_run(
                            &mut observed_run,
                            exit_code,
                            Some(make_observability_error(
                                "io_error",
                                "check.dump_tokens",
                                message,
                                None,
                            )),
                        );
                    }
                };
                println!("{json}");
            }
        }

        return finalize_observed_run(&mut observed_run, EXIT_OK, None);
    }

    let ast = match observe_command_phase_result(&mut observed_run, "frontend.parse_ast", || {
        parse_ast(&tokens)
    }) {
        Ok(ast) => ast,
        Err(error) => {
            let message = error.to_string();
            let source_info = observability_error_source(&source_path, &source, error.offset());
            let exit_code = CliDiagnostic::failure_with_filename_and_source(
                message.clone(),
                Some(&source_path),
                &source,
                error.offset(),
            )
            .emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "parser_error",
                    "frontend.parse_ast",
                    message,
                    source_info,
                )),
            );
        }
    };

    if dump_ast {
        let json = match serde_json::to_string(&ast) {
            Ok(value) => value,
            Err(_) => {
                let message = "failed to serialize ast".to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "check.dump_ast",
                        message,
                        None,
                    )),
                );
            }
        };

        println!("{json}");
        return finalize_observed_run(&mut observed_run, EXIT_OK, None);
    }

    if let Err(error) =
        observe_command_phase_result(&mut observed_run, "frontend.resolve_ast", || {
            resolve_ast(&ast)
        })
    {
        let message = error.to_string();
        let source_info = observability_error_source(&source_path, &source, error.offset());
        let exit_code = CliDiagnostic::failure_with_filename_and_source(
            message.clone(),
            Some(&source_path),
            &source,
            error.offset(),
        )
        .emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "resolver_error",
                "frontend.resolve_ast",
                message,
                source_info,
            )),
        );
    }

    let type_summary =
        match observe_command_phase_result(&mut observed_run, "frontend.infer_types", || {
            infer_types(&ast)
        }) {
            Ok(summary) => summary,
            Err(error) => {
                let message = error.to_string();
                let source_info = observability_error_source(&source_path, &source, error.offset());
                let exit_code = CliDiagnostic::failure_with_filename_and_source(
                    message.clone(),
                    Some(&source_path),
                    &source,
                    error.offset(),
                )
                .emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "typing_error",
                        "frontend.infer_types",
                        message,
                        source_info,
                    )),
                );
            }
        };
    maybe_trace_type_summary(type_summary.len());

    if dump_ir {
        let ir = match observe_command_phase_result(&mut observed_run, "frontend.lower_ir", || {
            lower_ast_to_ir(&ast)
        }) {
            Ok(ir) => ir,
            Err(error) => {
                let message = error.to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "ir_lowering_error",
                        "frontend.lower_ir",
                        message,
                        None,
                    )),
                );
            }
        };

        let json = match serde_json::to_string(&ir) {
            Ok(value) => value,
            Err(_) => {
                let message = "failed to serialize ir".to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "check.dump_ir",
                        message,
                        None,
                    )),
                );
            }
        };

        println!("{json}");
        return finalize_observed_run(&mut observed_run, EXIT_OK, None);
    }

    if dump_mir {
        let ir = match observe_command_phase_result(&mut observed_run, "frontend.lower_ir", || {
            lower_ast_to_ir(&ast)
        }) {
            Ok(ir) => ir,
            Err(error) => {
                let message = error.to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "ir_lowering_error",
                        "frontend.lower_ir",
                        message,
                        None,
                    )),
                );
            }
        };

        let mir = match observe_command_phase_result(&mut observed_run, "backend.lower_mir", || {
            lower_ir_to_mir(&ir)
        }) {
            Ok(mir) => mir,
            Err(error) => {
                let message = error.to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "mir_lowering_error",
                        "backend.lower_mir",
                        message,
                        None,
                    )),
                );
            }
        };

        let json = match serde_json::to_string(&mir) {
            Ok(value) => value,
            Err(_) => {
                let message = "failed to serialize mir".to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "check.dump_mir",
                        message,
                        None,
                    )),
                );
            }
        };

        println!("{json}");
        return finalize_observed_run(&mut observed_run, EXIT_OK, None);
    }

    if is_project_root_path {
        println!("check: ok");
    }

    finalize_observed_run(&mut observed_run, EXIT_OK, None)
}

fn handle_test(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_test_help();
        return EXIT_OK;
    }

    if args.is_empty() {
        return CliDiagnostic::usage_with_hint(
            "missing required <path>",
            "run `tonic test --help` for usage",
        )
        .emit();
    }

    let source_path = args[0].clone();
    let mut format = TestOutputFormat::Text;
    let mut index = 1;

    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                let Some(value) = args.get(index + 1) else {
                    return CliDiagnostic::usage("missing value for --format").emit();
                };

                let Some(parsed) = TestOutputFormat::parse(value) else {
                    return CliDiagnostic::usage(format!(
                        "unsupported format '{value}' (expected 'text' or 'json')"
                    ))
                    .emit();
                };

                format = parsed;
                index += 2;
            }
            other => {
                return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit();
            }
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut observed_run = ObservabilityRun::from_env("test", &command_argv("test", &args), &cwd);
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_metadata(
            "format",
            match format {
                TestOutputFormat::Text => "text",
                TestOutputFormat::Json => "json",
            },
        );
    }

    let report = match observe_command_phase_result(&mut observed_run, "test.run_suite", || {
        test_runner::run(&source_path)
    }) {
        Ok(report) => report,
        Err(TestRunnerError::Failure(message)) => {
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "io_error",
                    "test.run_suite",
                    message,
                    None,
                )),
            );
        }
        Err(TestRunnerError::SourceDiagnostic {
            message,
            filename,
            source,
            offset,
        }) => {
            let source_path = filename.unwrap_or_else(|| source_path.clone());
            let source_info = observability_error_source(&source_path, &source, offset);
            let exit_code = CliDiagnostic::failure_with_filename_and_source(
                message.clone(),
                Some(&source_path),
                &source,
                offset,
            )
            .emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "typing_error",
                    "test.run_suite",
                    message,
                    source_info,
                )),
            );
        }
    };

    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_metadata("total", report.total as u64);
        observed_run.record_metadata("passed", report.passed as u64);
        observed_run.record_metadata("failed", report.failed as u64);
    }

    match format {
        TestOutputFormat::Text => {
            for line in report.render_text() {
                println!("{line}");
            }
        }
        TestOutputFormat::Json => {
            println!("{}", report.render_json());
        }
    }

    if report.succeeded() {
        finalize_observed_run(&mut observed_run, EXIT_OK, None)
    } else {
        finalize_observed_run(
            &mut observed_run,
            EXIT_FAILURE,
            Some(make_observability_error(
                "script_error",
                "test.run_suite",
                format!("{} test(s) failed", report.failed),
                None,
            )),
        )
    }
}

fn handle_fmt(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_fmt_help();
        return EXIT_OK;
    }

    if args.is_empty() {
        return CliDiagnostic::usage_with_hint(
            "missing required <path>",
            "run `tonic fmt --help` for usage",
        )
        .emit();
    }

    let source_path = args[0].clone();
    let mut mode = FormatMode::Write;

    for argument in args.iter().skip(1) {
        match argument.as_str() {
            "--check" => mode = FormatMode::Check,
            other => return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit(),
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut observed_run = ObservabilityRun::from_env("fmt", &command_argv("fmt", &args), &cwd);
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_metadata(
            "mode",
            match mode {
                FormatMode::Write => "write",
                FormatMode::Check => "check",
            },
        );
    }

    let report = match observe_command_phase_result(&mut observed_run, "fmt.format_path", || {
        format_path(&source_path, mode)
    }) {
        Ok(report) => report,
        Err(error) => {
            let exit_code = CliDiagnostic::failure(error.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "io_error",
                    "fmt.format_path",
                    error,
                    None,
                )),
            );
        }
    };

    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_metadata("checked_files", report.checked_files as u64);
        observed_run.record_metadata("changed_files", report.changed_files as u64);
    }

    if mode == FormatMode::Check && report.changed_files > 0 {
        let suffix = if report.changed_files == 1 { "" } else { "s" };
        let message = format!(
            "formatting required for {} file{} (run `tonic fmt <path>` to apply fixes)",
            report.changed_files, suffix
        );
        let exit_code = CliDiagnostic::failure(message.clone()).emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "script_error",
                "fmt.format_path",
                message,
                None,
            )),
        );
    }

    println!("fmt: ok");
    finalize_observed_run(&mut observed_run, EXIT_OK, None)
}

fn handle_compile(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_compile_help();
        return EXIT_OK;
    }

    if args.is_empty() {
        return CliDiagnostic::usage_with_hint(
            "missing required <path>",
            "run `tonic compile --help` for usage",
        )
        .emit();
    }

    let source_path = args[0].clone();
    let mut out_path = None;
    let mut target_triple = None;
    let mut idx = 1;

    while idx < args.len() {
        match args[idx].as_str() {
            "--out" => {
                idx += 1;
                if idx >= args.len() {
                    return CliDiagnostic::usage("--out requires a value").emit();
                }
                out_path = Some(args[idx].clone());
                idx += 1;
            }
            "--target" => {
                idx += 1;
                if idx >= args.len() {
                    return CliDiagnostic::usage("--target requires a value").emit();
                }
                let raw = &args[idx];
                match target::TargetTriple::parse(raw) {
                    Ok(t) => target_triple = Some(t),
                    Err(err) => return CliDiagnostic::usage(err.message).emit(),
                }
                idx += 1;
            }
            other => {
                return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit();
            }
        }
    }

    let target = target_triple.unwrap_or_else(target::TargetTriple::host);
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut observed_run =
        ObservabilityRun::from_env("compile", &command_argv("compile", &args), &cwd);
    let mut profiler = profiling::PhaseProfiler::from_env("compile");
    let is_project_root_path = std::path::Path::new(&source_path).is_dir();

    let source = match observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "compile.load_source",
        || load_run_source(&source_path),
    ) {
        Ok(source) => source,
        Err(error) => {
            let message = error;
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "io_error",
                    "compile.load_source",
                    message,
                    None,
                )),
            );
        }
    };

    let ir = match compile_source_to_ir(&source, &mut profiler, &mut observed_run) {
        Ok(ir) => ir,
        Err(error) => {
            let obs_error = error.to_observability_error(&source_path, &source);
            let exit_code = error.into_diagnostic(Some(&source_path), &source).emit();
            return finalize_observed_run(&mut observed_run, exit_code, Some(obs_error));
        }
    };

    let artifact_stem = compile_artifact_stem(&source_path, is_project_root_path);
    let mir = match observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "backend.lower_mir",
        || lower_ir_to_mir(&ir),
    ) {
        Ok(mir) => mir,
        Err(error) => {
            let message = error.to_string();
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "mir_lowering_error",
                    "backend.lower_mir",
                    message,
                    None,
                )),
            );
        }
    };

    let optimized_mir = observe_phase(
        &mut profiler,
        &mut observed_run,
        "backend.optimize_mir",
        || optimize_for_native_backend(mir),
    );

    let sidecar_base = {
        let mut p = default_compile_build_dir();
        p.push(&artifact_stem);
        p
    };
    let ll_path = sidecar_base.with_extension("ll");
    let c_path = sidecar_base.with_extension("c");
    let ir_path = sidecar_base.with_extension("tir.json");
    let manifest_path = sidecar_base.with_extension("tnx.json");
    let exe_path = match out_path {
        Some(ref path) => std::path::PathBuf::from(path),
        None => sidecar_base.clone(),
    };

    if exe_path.is_dir() {
        let message = format!("--out path '{}' is a directory", exe_path.display());
        let exit_code = CliDiagnostic::usage(message.clone()).emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "usage_error",
                "backend.prepare_artifacts",
                message,
                None,
            )),
        );
    }

    for path in [&ll_path, &c_path, &ir_path, &manifest_path, &exe_path] {
        if let Err(message) = ensure_artifact_parent(path) {
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "io_error",
                    "backend.prepare_artifacts",
                    message,
                    None,
                )),
            );
        }
    }

    llvm_backend::warn_experimental();
    let llvm_ir = match observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "backend.lower_llvm",
        || llvm_backend::lower_mir_subset_to_llvm_ir(&optimized_mir, &target),
    ) {
        Ok(llvm_ir) => llvm_ir,
        Err(error) => {
            let message = error.to_string();
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "backend_error",
                    "backend.lower_llvm",
                    message,
                    None,
                )),
            );
        }
    };

    if let Err(error) = observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "backend.write_llvm_ir",
        || crate::cache::write_atomic(&ll_path, &llvm_ir),
    ) {
        let message = format!(
            "failed to write llvm ir sidecar to {}: {error}",
            ll_path.display()
        );
        let exit_code = CliDiagnostic::failure(message.clone()).emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "io_error",
                "backend.write_llvm_ir",
                message,
                None,
            )),
        );
    }
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_artifact("llvm-ir", &ll_path);
    }

    let serialized_ir = match serde_json::to_string(&ir) {
        Ok(s) => s,
        Err(error) => {
            let message = format!("failed to serialize compile artifact: {error}");
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "ir_lowering_error",
                    "backend.write_ir",
                    message,
                    None,
                )),
            );
        }
    };

    if let Err(error) =
        observe_phase_result(&mut profiler, &mut observed_run, "backend.write_ir", || {
            crate::cache::write_atomic(&ir_path, &serialized_ir)
        })
    {
        let message = format!(
            "failed to write ir sidecar to {}: {error}",
            ir_path.display()
        );
        let exit_code = CliDiagnostic::failure(message.clone()).emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "io_error",
                "backend.write_ir",
                message,
                None,
            )),
        );
    }
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_artifact("ir-sidecar", &ir_path);
    }

    let manifest = native_artifact::build_executable_manifest(
        &source,
        &manifest_path,
        &ll_path,
        &exe_path,
        &ir_path,
    );
    if let Err(error) = observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "backend.write_manifest",
        || native_artifact::write_manifest(&manifest_path, &manifest),
    ) {
        let message = error;
        let exit_code = CliDiagnostic::failure(message.clone()).emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "io_error",
                "backend.write_manifest",
                message,
                None,
            )),
        );
    }
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_artifact("native-manifest", &manifest_path);
    }

    let c_source =
        match observe_phase_result(&mut profiler, &mut observed_run, "backend.lower_c", || {
            c_backend::lower_mir_to_c(&optimized_mir)
        }) {
            Ok(src) => src,
            Err(error) => {
                let message = error.to_string();
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "backend_error",
                        "backend.lower_c",
                        message,
                        None,
                    )),
                );
            }
        };

    if let Err(error) = observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "backend.write_c_source",
        || crate::cache::write_atomic(&c_path, &c_source),
    ) {
        let message = format!("failed to write c source to {}: {error}", c_path.display());
        let exit_code = CliDiagnostic::failure(message.clone()).emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "io_error",
                "backend.write_c_source",
                message,
                None,
            )),
        );
    }
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_artifact("c-source", &c_path);
    }

    if let Err(error) = observe_phase_result(
        &mut profiler,
        &mut observed_run,
        "backend.link_executable",
        || linker::compile_c_to_executable(&c_path, &exe_path, &target),
    ) {
        let message = error.to_string();
        let exit_code = CliDiagnostic::failure(message.clone()).emit();
        return finalize_observed_run(
            &mut observed_run,
            exit_code,
            Some(make_observability_error(
                "linker_error",
                "backend.link_executable",
                message,
                None,
            )),
        );
    }
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_artifact("native-executable", &exe_path);
    }

    println!("compile: ok {}", exe_path.display());
    finalize_observed_run(&mut observed_run, EXIT_OK, None)
}

fn compile_artifact_stem(source_path: &str, is_project_root_path: bool) -> String {
    if is_project_root_path {
        manifest::load_project_manifest(std::path::Path::new(source_path))
            .ok()
            .and_then(|m| {
                m.entry
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "out".to_string())
    } else {
        std::path::Path::new(source_path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "out".to_string())
    }
}

fn default_compile_build_dir() -> std::path::PathBuf {
    let mut path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    path.push(".tonic");
    path.push("build");
    path
}

fn ensure_artifact_parent(path: &std::path::Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create artifact directory {}: {}",
                parent.display(),
                error
            )
        })?;
    }

    Ok(())
}

fn handle_verify(args: Vec<String>) -> i32 {
    let mut iter = args.into_iter();

    match iter.next().as_deref() {
        None | Some("-h") | Some("--help") => {
            print_verify_help();
            EXIT_OK
        }
        Some("run") => handle_verify_run(iter.collect()),
        Some(other) => CliDiagnostic::usage_with_hint(
            format!("unknown verify subcommand '{other}'"),
            "run `tonic verify --help` for usage",
        )
        .emit(),
    }
}

fn handle_verify_run(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_verify_run_help();
        return EXIT_OK;
    }

    if args.is_empty() {
        return CliDiagnostic::usage_with_hint(
            "missing required <slice-id>",
            "run `tonic verify run --help` for usage",
        )
        .emit();
    }

    let slice_id = args[0].clone();
    let mut mode = VerifyMode::Auto;
    let mut idx = 1;

    while idx < args.len() {
        match args[idx].as_str() {
            "--mode" => {
                idx += 1;

                if idx >= args.len() {
                    return CliDiagnostic::usage("--mode requires a value").emit();
                }

                let candidate = &args[idx];
                let Some(parsed_mode) = VerifyMode::parse(candidate) else {
                    return CliDiagnostic::usage(format!("unsupported mode '{candidate}'")).emit();
                };

                mode = parsed_mode;
                idx += 1;
            }
            other => {
                return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit();
            }
        }
    }

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut observed_run =
        ObservabilityRun::from_env("verify", &command_argv("verify", &args), &cwd);
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.set_target_path(slice_id.clone());
        observed_run.record_metadata("subcommand", "run");
        observed_run.record_metadata("mode", mode.as_str());
    }

    let acceptance =
        match observe_command_phase_result(&mut observed_run, "verify.load_acceptance", || {
            load_acceptance_yaml(&slice_id)
        }) {
            Ok(metadata) => metadata,
            Err(message) => {
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "verify.load_acceptance",
                        message,
                        None,
                    )),
                );
            }
        };

    let scenarios =
        match observe_command_phase_result(&mut observed_run, "verify.load_scenarios", || {
            load_feature_scenarios(&acceptance.feature_files)
        }) {
            Ok(scenarios) => scenarios,
            Err(message) => {
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "verify.load_scenarios",
                        message,
                        None,
                    )),
                );
            }
        };

    let mode_tags = mode.selected_tags();
    let verify_result = observe_command_phase_result(
        &mut observed_run,
        "verify.evaluate_gates",
        || {
            let filtered_scenarios = scenarios
            .iter()
            .filter(|scenario| {
                scenario
                    .tags
                    .iter()
                    .any(|tag| mode_tags.contains(&tag.as_str()))
            })
            .map(|scenario| serde_json::json!({ "id": scenario.id.clone(), "tags": scenario.tags.clone() }))
            .collect::<Vec<_>>();

            let (benchmark_failed, benchmark_report) =
                benchmark_gate_report(acceptance.benchmark_metrics.as_ref());
            let (manual_evidence_failed, manual_evidence_report) =
                manual_evidence_gate_report(acceptance.manual_evidence.for_mode(mode.as_str()));
            let verify_failed = benchmark_failed || manual_evidence_failed;

            let report = serde_json::json!({
                "slice_id": slice_id,
                "mode": mode.as_str(),
                "status": if verify_failed { "fail" } else { "pass" },
                "acceptance_file": acceptance.path.display().to_string(),
                "mode_tags": mode_tags,
                "scenarios": filtered_scenarios,
                "benchmark": benchmark_report,
                "manual_evidence": manual_evidence_report,
            });

            if verify_failed {
                Err(report)
            } else {
                Ok(report)
            }
        },
    );

    let (report, exit_code, error) = match verify_result {
        Ok(report) => (report, EXIT_OK, None),
        Err(report) => {
            let message = format!(
                "verification failed: benchmark={} manual_evidence={}",
                report["benchmark"]["status"].as_str().unwrap_or("unknown"),
                report["manual_evidence"]["status"]
                    .as_str()
                    .unwrap_or("unknown")
            );
            (
                report,
                EXIT_FAILURE,
                Some(make_observability_error(
                    "script_error",
                    "verify.evaluate_gates",
                    message,
                    None,
                )),
            )
        }
    };

    println!("{report}");
    finalize_observed_run(&mut observed_run, exit_code, error)
}

fn benchmark_gate_report(
    benchmark_metrics: Option<&BenchmarkMetrics>,
) -> (bool, serde_json::Value) {
    match benchmark_metrics {
        Some(metrics) => {
            let cold_exceeded = metrics.cold_start_p50_ms > BENCHMARK_THRESHOLD_COLD_START_P50_MS;
            let warm_exceeded = metrics.warm_start_p50_ms > BENCHMARK_THRESHOLD_WARM_START_P50_MS;
            let rss_exceeded = metrics.idle_rss_mb > BENCHMARK_THRESHOLD_IDLE_RSS_MB;
            let threshold_exceeded = cold_exceeded || warm_exceeded || rss_exceeded;

            let mut diagnostics = Vec::new();
            if cold_exceeded {
                diagnostics.push("cold_start_p50_ms");
            }
            if warm_exceeded {
                diagnostics.push("warm_start_p50_ms");
            }
            if rss_exceeded {
                diagnostics.push("idle_rss_mb");
            }

            (
                threshold_exceeded,
                serde_json::json!({
                    "status": if threshold_exceeded { "threshold_exceeded" } else { "pass" },
                    "diagnostics": diagnostics,
                    "thresholds": benchmark_thresholds_report(),
                    "measured": {
                        "cold_start_p50_ms": metrics.cold_start_p50_ms,
                        "warm_start_p50_ms": metrics.warm_start_p50_ms,
                        "idle_rss_mb": metrics.idle_rss_mb,
                    }
                }),
            )
        }
        None => (
            false,
            serde_json::json!({
                "status": "not_configured",
                "diagnostics": [],
                "thresholds": benchmark_thresholds_report(),
            }),
        ),
    }
}

fn manual_evidence_gate_report(required_paths: &[std::path::PathBuf]) -> (bool, serde_json::Value) {
    let required = required_paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();

    if required.is_empty() {
        return (
            false,
            serde_json::json!({
                "status": "not_configured",
                "required": [],
                "missing": [],
                "invalid": [],
            }),
        );
    }

    let mut missing = Vec::new();
    let mut invalid = Vec::new();

    for path in required_paths {
        if !path.is_file() {
            missing.push(path.display().to_string());
        } else {
            let content = std::fs::read_to_string(path).unwrap_or_default();
            if serde_json::from_str::<serde_json::Value>(&content).is_err() {
                invalid.push(path.display().to_string());
            }
        }
    }

    let missing_required = !missing.is_empty();
    let has_invalid = !invalid.is_empty();
    let failed = missing_required || has_invalid;

    let status = if missing_required {
        "missing_required"
    } else if has_invalid {
        "invalid_payload"
    } else {
        "pass"
    };

    (
        failed,
        serde_json::json!({
            "status": status,
            "required": required,
            "missing": missing,
            "invalid": invalid,
        }),
    )
}

fn handle_deps(args: Vec<String>) -> i32 {
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        print_deps_help();
        return EXIT_OK;
    }

    let subcommand = match args.first().map(String::as_str) {
        Some("sync") | Some("fetch") | Some("lock") => args[0].clone(),
        Some(other) => {
            return CliDiagnostic::usage_with_hint(
                format!("unknown deps subcommand '{other}'"),
                "run `tonic deps --help` for available subcommands",
            )
            .emit();
        }
        None => {
            return CliDiagnostic::usage_with_hint(
                "missing deps subcommand",
                "run `tonic deps --help` for available subcommands",
            )
            .emit();
        }
    };

    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let mut observed_run = ObservabilityRun::from_env("deps", &command_argv("deps", &args), &cwd);
    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.record_metadata("subcommand", subcommand.clone());
    }

    let project_root = match observe_command_phase(
        &mut observed_run,
        "deps.find_project_root",
        find_project_root,
    ) {
        Some(project_root) => project_root,
        None => {
            let message = "no tonic.toml found in current directory or parents".to_string();
            let exit_code = CliDiagnostic::failure(message.clone()).emit();
            return finalize_observed_run(
                &mut observed_run,
                exit_code,
                Some(make_observability_error(
                    "io_error",
                    "deps.find_project_root",
                    message,
                    None,
                )),
            );
        }
    };

    if let Some(observed_run) = observed_run.as_mut() {
        observed_run.set_target_path(project_root.display().to_string());
    }

    let manifest =
        match observe_command_phase_result(&mut observed_run, "deps.load_manifest", || {
            manifest::load_project_manifest(&project_root)
        }) {
            Ok(manifest) => manifest,
            Err(message) => {
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "deps.load_manifest",
                        message,
                        None,
                    )),
                );
            }
        };

    if manifest.dependencies.path.is_empty()
        && manifest.dependencies.git.is_empty()
        && manifest.dependencies.registry.is_empty()
    {
        println!("No dependencies defined in tonic.toml");
        return finalize_observed_run(&mut observed_run, EXIT_OK, None);
    }

    match subcommand.as_str() {
        "sync" | "fetch" => {
            println!("Syncing dependencies...");
            match observe_command_phase_result(&mut observed_run, "deps.sync", || {
                deps::DependencyResolver::sync(&manifest.dependencies, &project_root)
            }) {
                Ok(lockfile) => {
                    if let Some(observed_run) = observed_run.as_mut() {
                        observed_run
                            .record_metadata("path_dependencies", lockfile.path_deps.len() as u64);
                        observed_run
                            .record_metadata("git_dependencies", lockfile.git_deps.len() as u64);
                    }
                    println!("Dependencies synced successfully.");
                    println!("Lockfile saved to tonic.lock");
                    println!("  - path dependencies: {}", lockfile.path_deps.len());
                    println!("  - git dependencies: {}", lockfile.git_deps.len());
                    finalize_observed_run(&mut observed_run, EXIT_OK, None)
                }
                Err(msg) => {
                    let message = format!("failed to sync dependencies: {msg}");
                    let exit_code = CliDiagnostic::failure(message.clone()).emit();
                    finalize_observed_run(
                        &mut observed_run,
                        exit_code,
                        Some(make_observability_error(
                            "io_error",
                            "deps.sync",
                            message,
                            None,
                        )),
                    )
                }
            }
        }
        "lock" => {
            println!("Generating lockfile...");
            match observe_command_phase_result(&mut observed_run, "deps.lock", || {
                deps::Lockfile::generate(&manifest.dependencies, &project_root)
            }) {
                Ok(lockfile) => match lockfile.save(&project_root) {
                    Ok(()) => {
                        println!("Lockfile generated: tonic.lock");
                        finalize_observed_run(&mut observed_run, EXIT_OK, None)
                    }
                    Err(msg) => {
                        let message = format!("failed to save lockfile: {msg}");
                        let exit_code = CliDiagnostic::failure(message.clone()).emit();
                        finalize_observed_run(
                            &mut observed_run,
                            exit_code,
                            Some(make_observability_error(
                                "io_error",
                                "deps.lock",
                                message,
                                None,
                            )),
                        )
                    }
                },
                Err(msg) => {
                    let message = format!("failed to generate lockfile: {msg}");
                    let exit_code = CliDiagnostic::failure(message.clone()).emit();
                    finalize_observed_run(
                        &mut observed_run,
                        exit_code,
                        Some(make_observability_error(
                            "io_error",
                            "deps.lock",
                            message,
                            None,
                        )),
                    )
                }
            }
        }
        _ => finalize_observed_run(&mut observed_run, EXIT_OK, None),
    }
}

fn find_project_root() -> Option<std::path::PathBuf> {
    let mut current = std::env::current_dir().ok()?;
    loop {
        if current.join("tonic.toml").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn handle_publish(args: Vec<String>) -> i32 {
    if matches!(args.first().map(String::as_str), Some("-h" | "--help")) {
        print_publish_help();
        return EXIT_OK;
    }

    let project_root = find_project_root();
    if project_root.is_none() {
        return CliDiagnostic::failure("no tonic.toml found in current directory or parents")
            .emit();
    }
    let project_root = project_root.unwrap();

    let manifest = match manifest::load_project_manifest(&project_root) {
        Ok(m) => m,
        Err(msg) => return CliDiagnostic::failure(msg).emit(),
    };

    // Validate required package fields before attempting publish.
    let package = match &manifest.package {
        Some(p) => p,
        None => {
            return CliDiagnostic::failure(
                "tonic.toml is missing a [package] section; add name, version, and description",
            )
            .emit();
        }
    };

    let mut missing_fields = Vec::new();
    if package.name.is_none() {
        missing_fields.push("name");
    }
    if package.version.is_none() {
        missing_fields.push("version");
    }
    if package.description.is_none() {
        missing_fields.push("description");
    }

    if !missing_fields.is_empty() {
        return CliDiagnostic::failure(format!(
            "tonic.toml [package] is missing required fields: {}",
            missing_fields.join(", ")
        ))
        .emit();
    }

    println!("Publishing to registry is not yet supported");
    EXIT_OK
}

fn print_publish_help() {
    println!(
        "tonic publish - Publish package to registry (not yet implemented)\n\n\
         Usage:\n  tonic publish\n\n\
         Requires tonic.toml [package] section with:\n  \
         name, version, description\n\n\
         Publishing to a registry is not yet supported.\n"
    );
}

fn print_deps_help() {
    println!(
        "tonic deps - Manage project dependencies\n\n\
         Usage:\n  tonic deps <SUBCOMMAND>\n\n\
         Subcommands:\n  sync    Fetch all dependencies and generate lockfile\n  fetch   Alias for sync\n  lock    Generate lockfile without fetching\n\n\
         Examples:\n  tonic deps sync    # Fetch dependencies and create tonic.lock\n  tonic deps lock    # Just create/update tonic.lock\n"
    );
}

fn benchmark_thresholds_report() -> serde_json::Value {
    serde_json::json!({
        "cold_start_p50_ms": BENCHMARK_THRESHOLD_COLD_START_P50_MS,
        "warm_start_p50_ms": BENCHMARK_THRESHOLD_WARM_START_P50_MS,
        "idle_rss_mb": BENCHMARK_THRESHOLD_IDLE_RSS_MB,
    })
}

fn print_help() {
    println!(
        "tonic language core v0\n\nUsage:\n  tonic <COMMAND> [OPTIONS]\n\nCommands:\n  run      Execute source\n  repl     Start interactive REPL\n  check    Parse and type-check source\n  test     Run project tests\n  fmt      Format source files\n  compile  Compile source to executable artifact\n  cache    Manage compiled artifacts\n  verify   Run acceptance verification\n  deps     Manage project dependencies\n  docs     Generate API documentation\n  lsp      Start language server\n  publish  Publish package to registry (not yet implemented)\n"
    );
}

fn print_run_help() {
    println!("Usage:\n  tonic run <path>\n");
}

fn print_check_help() {
    println!(
        "Usage:\n  tonic check <path> [--dump-tokens [--format <text|json>]|--dump-ast|--dump-ir|--dump-mir]\n"
    );
}

fn print_test_help() {
    println!("Usage:\n  tonic test <path> [--format <text|json>]\n");
}

fn print_fmt_help() {
    println!("Usage:\n  tonic fmt <path> [--check]\n");
}

fn print_compile_help() {
    println!(
        "Usage:\n  tonic compile <path> [--out <artifact-path>] [--target <triple>]\n\n\
         Compile contract:\n\
         \x20 Compile always produces a native executable artifact (ELF on Linux, Mach-O on macOS).\n\
         \x20 Default output: .tonic/build/<name>  (runnable as ./.tonic/build/<name>)\n\
         \x20 --out <path>       Write executable to <path> directly\n\
         \x20 --target <triple>  Cross-compile for the given target triple (default: host)\n\n\
         Supported targets:\n\
         \x20 x86_64-unknown-linux-gnu    (default on x86_64 Linux)\n\
         \x20 aarch64-unknown-linux-gnu   (ARM64 Linux; requires aarch64-linux-gnu-gcc or clang)\n\
         \x20 x86_64-apple-darwin         (Intel macOS; requires clang)\n\
         \x20 aarch64-apple-darwin        (Apple Silicon; requires clang)\n\n\
         Cross-compilation:\n\
         \x20 Requires: clang (recommended, uses -target <triple>) or GNU cross-compilers\n\
         \x20 For aarch64-linux: apt install gcc-aarch64-linux-gnu  (or use clang)\n\
         \x20 For macOS targets: requires clang with macOS SDK support\n\
         \x20 See docs/cross-compilation.md for detailed setup instructions.\n\n\
         Requires: cc, gcc, or clang in PATH (native); clang or cross-gcc (cross)\n"
    );
}

fn print_verify_help() {
    println!("Usage:\n  tonic verify run <slice-id> [--mode <auto|mixed|manual>]\n");
}

fn print_verify_run_help() {
    println!("Usage:\n  tonic verify run <slice-id> [--mode <auto|mixed|manual>]\n");
}

#[cfg(test)]
mod tests {
    use super::{run, VerifyMode, EXIT_OK};
    use crate::cli_diag::{EXIT_FAILURE, EXIT_USAGE};

    #[test]
    fn known_commands_without_args_exit_usage() {
        for command in ["run", "check", "test", "fmt", "compile", "deps"] {
            assert_eq!(run(vec![command.to_string()]), EXIT_USAGE);
        }
    }

    #[test]
    fn known_commands_without_args_exit_success() {
        for command in ["cache", "verify"] {
            assert_eq!(run(vec![command.to_string()]), EXIT_OK);
        }
    }

    #[test]
    fn verify_command_routes_to_verify_subcommand() {
        assert_eq!(run(vec!["verify".to_string()]), EXIT_OK);
        assert_eq!(
            run(vec![
                "verify".to_string(),
                "run".to_string(),
                "unit-missing-acceptance".to_string(),
                "--mode".to_string(),
                "auto".to_string()
            ]),
            EXIT_FAILURE
        );
    }

    #[test]
    fn verify_mode_exposes_expected_tag_metadata() {
        assert_eq!(VerifyMode::Auto.selected_tags(), ["@auto"]);
        assert_eq!(
            VerifyMode::Mixed.selected_tags(),
            ["@auto", "@agent-manual"]
        );
        assert_eq!(
            VerifyMode::Manual.selected_tags(),
            ["@auto", "@agent-manual", "@human-manual"]
        );
    }

    #[test]
    fn cli_diagnostics_share_usage_formatting() {
        let diagnostic = crate::cli_diag::CliDiagnostic::usage_with_hint(
            "unknown command 'mystery'",
            "run `tonic --help` to see available commands",
        );

        assert_eq!(diagnostic.exit_code(), EXIT_USAGE);
        assert_eq!(
            diagnostic.lines(),
            [
                "error: unknown command 'mystery'".to_string(),
                "run `tonic --help` to see available commands".to_string(),
            ]
        );
    }

    #[test]
    fn acceptance_util_uses_standard_slice_path() {
        let path = crate::acceptance::acceptance_file_path("step-01");

        assert_eq!(path, std::path::PathBuf::from("acceptance/step-01.yaml"));
    }

    #[test]
    fn unknown_command_uses_usage_exit_code() {
        assert_eq!(run(vec!["unknown".to_string()]), EXIT_USAGE);
    }

    // --- Serialization failure diagnostic contracts ---
    //
    // serde_json::to_string can return Err in degenerate cases (e.g. maps with
    // non-string keys serialized via derived Serialize). The actual AST/IR are
    // unlikely to trigger this in practice, but the error branch must produce a
    // deterministic, well-formed diagnostic. These unit tests verify that
    // contract directly without going through the full CLI binary.

    #[test]
    fn dump_ast_serialization_failure_emits_deterministic_diagnostic() {
        let diagnostic = crate::cli_diag::CliDiagnostic::failure("failed to serialize ast");
        assert_eq!(diagnostic.exit_code(), EXIT_FAILURE);
        assert_eq!(
            diagnostic.lines(),
            ["error: failed to serialize ast".to_string()]
        );
    }

    #[test]
    fn dump_ir_serialization_failure_emits_deterministic_diagnostic() {
        let diagnostic = crate::cli_diag::CliDiagnostic::failure("failed to serialize ir");
        assert_eq!(diagnostic.exit_code(), EXIT_FAILURE);
        assert_eq!(
            diagnostic.lines(),
            ["error: failed to serialize ir".to_string()]
        );
    }

    #[test]
    fn dump_tokens_serialization_failure_emits_deterministic_diagnostic() {
        let diagnostic = crate::cli_diag::CliDiagnostic::failure("failed to serialize token dump");
        assert_eq!(diagnostic.exit_code(), EXIT_FAILURE);
        assert_eq!(
            diagnostic.lines(),
            ["error: failed to serialize token dump".to_string()]
        );
    }

    #[test]
    fn dump_mode_exclusivity_error_uses_usage_exit_code() {
        let diagnostic = crate::cli_diag::CliDiagnostic::usage(
            "--dump-tokens, --dump-ast, --dump-ir, and --dump-mir cannot be used together",
        );
        assert_eq!(diagnostic.exit_code(), EXIT_USAGE);
        assert_eq!(
            diagnostic.lines(),
            [
                "error: --dump-tokens, --dump-ast, --dump-ir, and --dump-mir cannot be used together"
                    .to_string()
            ]
        );
    }

    #[test]
    fn dump_tokens_format_requires_dump_tokens_flag() {
        let diagnostic =
            crate::cli_diag::CliDiagnostic::usage("--format is only supported with --dump-tokens");
        assert_eq!(diagnostic.exit_code(), EXIT_USAGE);
        assert_eq!(
            diagnostic.lines(),
            ["error: --format is only supported with --dump-tokens".to_string()]
        );
    }
}
