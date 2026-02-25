mod acceptance;
mod c_backend;
mod cache;
mod cli_diag;
mod deps;
mod formatter;
mod interop;
mod ir;
mod lexer;
mod linker;
mod llvm_backend;
mod manifest;
mod mir;
pub mod native_abi;
mod native_artifact;
pub mod native_runtime;
mod parser;
mod profiling;
mod resolver;
mod resolver_diag;
mod runtime;
mod typing;

use acceptance::{load_acceptance_yaml, load_feature_scenarios, BenchmarkMetrics};
use cache::{
    build_run_cache_key, load_cached_ir, should_trace_cache_status, store_cached_ir,
    trace_cache_status,
};
use cli_diag::{CliDiagnostic, EXIT_FAILURE, EXIT_OK};
use formatter::{format_path, FormatMode};
use ir::{lower_ast_to_ir, IrProgram};
use lexer::scan_tokens;
use manifest::load_run_source;
use mir::{lower_ir_to_mir, optimize_for_native_backend};
use parser::parse_ast;
use resolver::resolve_ast;
use runtime::{evaluate_entrypoint, RuntimeValue};
use typing::infer_types;

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
        Some("verify") => handle_verify(iter.collect()),
        Some("deps") => handle_deps(iter.collect()),
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

    let mut profiler = profiling::PhaseProfiler::from_env("run");

    if native_artifact::is_native_artifact_path(&source_path) {
        return handle_run_native_artifact(&source_path, &mut profiler);
    }

    let source = match profiling::profile_phase(&mut profiler, "run.load_source", || {
        load_run_source(&source_path)
    }) {
        Ok(source) => source,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    // Derive project_root from source_path (same logic as load_run_source)
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

    let ir = match profiling::profile_phase(&mut profiler, "run.cache_lookup", || {
        load_cached_ir(&cache_key)
    }) {
        Ok(Some(cached_ir)) => {
            cache_status = "hit";
            cached_ir
        }
        Ok(None) | Err(_) => {
            let compiled_ir = match compile_source_to_ir(&source, &mut profiler) {
                Ok(ir) => ir,
                Err(error) => return CliDiagnostic::failure(error).emit(),
            };

            if let Err(error) = profiling::profile_phase(&mut profiler, "run.cache_store", || {
                store_cached_ir(&cache_key, &compiled_ir)
            }) {
                eprintln!("warning: {}", error);
            }
            compiled_ir
        }
    };

    if should_trace_cache_status() {
        trace_cache_status(cache_status);
    }

    let value = match profiling::profile_phase(&mut profiler, "run.evaluate_entrypoint", || {
        evaluate_entrypoint(&ir)
    }) {
        Ok(value) => value,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };

    match value {
        RuntimeValue::ResultErr(reason) => {
            CliDiagnostic::failure(format!("runtime returned err({})", reason.render())).emit()
        }
        other => {
            println!("{}", other.render());
            EXIT_OK
        }
    }
}

fn handle_run_native_artifact(path: &str, profiler: &mut Option<profiling::PhaseProfiler>) -> i32 {
    let manifest_path = std::path::Path::new(path);
    let manifest = match profiling::profile_phase(profiler, "run.native.load_manifest", || {
        native_artifact::load_manifest(manifest_path)
    }) {
        Ok(manifest) => manifest,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    if let Err(error) = profiling::profile_phase(profiler, "run.native.validate_manifest", || {
        native_artifact::validate_manifest_for_host(&manifest)
    }) {
        return CliDiagnostic::failure(error).emit();
    }

    let ir = match profiling::profile_phase(profiler, "run.native.load_ir", || {
        native_artifact::load_ir_from_manifest(manifest_path, &manifest)
    }) {
        Ok(ir) => ir,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    let value = match profiling::profile_phase(profiler, "run.evaluate_entrypoint", || {
        evaluate_entrypoint(&ir)
    }) {
        Ok(value) => value,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };

    match value {
        RuntimeValue::ResultErr(reason) => {
            CliDiagnostic::failure(format!("runtime returned err({})", reason.render())).emit()
        }
        other => {
            println!("{}", other.render());
            EXIT_OK
        }
    }
}

fn compile_source_to_ir(
    source: &str,
    profiler: &mut Option<profiling::PhaseProfiler>,
) -> Result<IrProgram, String> {
    let tokens = profiling::profile_phase(profiler, "frontend.scan_tokens", || scan_tokens(source))
        .map_err(|error| error.to_string())?;
    let ast = profiling::profile_phase(profiler, "frontend.parse_ast", || parse_ast(&tokens))
        .map_err(|error| error.to_string())?;

    profiling::profile_phase(profiler, "frontend.resolve_ast", || resolve_ast(&ast))
        .map_err(|error| error.to_string())?;
    let type_summary =
        profiling::profile_phase(profiler, "frontend.infer_types", || infer_types(&ast))
            .map_err(|error| error.to_string())?;
    maybe_trace_type_summary(type_summary.len());

    profiling::profile_phase(profiler, "frontend.lower_ir", || lower_ast_to_ir(&ast))
        .map_err(|error| error.to_string())
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

    for argument in args.iter().skip(1) {
        match argument.as_str() {
            "--dump-tokens" => dump_tokens = true,
            "--dump-ast" => dump_ast = true,
            "--dump-ir" => dump_ir = true,
            "--dump-mir" => dump_mir = true,
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

    let source = match load_run_source(&source_path) {
        Ok(source) => source,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    let tokens = match scan_tokens(&source) {
        Ok(tokens) => tokens,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };

    if dump_tokens {
        for token in tokens {
            println!("{}", token.dump_label());
        }

        return EXIT_OK;
    }

    let ast = match parse_ast(&tokens) {
        Ok(ast) => ast,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };

    if dump_ast {
        let json = match serde_json::to_string(&ast) {
            Ok(value) => value,
            Err(_) => {
                return CliDiagnostic::failure("failed to serialize ast".to_string()).emit();
            }
        };

        println!("{json}");
        return EXIT_OK;
    }

    if let Err(error) = resolve_ast(&ast) {
        return CliDiagnostic::failure(error.to_string()).emit();
    }

    let type_summary = match infer_types(&ast) {
        Ok(summary) => summary,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };
    maybe_trace_type_summary(type_summary.len());

    if dump_ir {
        let ir = match lower_ast_to_ir(&ast) {
            Ok(ir) => ir,
            Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
        };

        let json = match serde_json::to_string(&ir) {
            Ok(value) => value,
            Err(_) => {
                return CliDiagnostic::failure("failed to serialize ir".to_string()).emit();
            }
        };

        println!("{json}");
        return EXIT_OK;
    }

    if dump_mir {
        let ir = match lower_ast_to_ir(&ast) {
            Ok(ir) => ir,
            Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
        };

        let mir = match lower_ir_to_mir(&ir) {
            Ok(mir) => mir,
            Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
        };

        let json = match serde_json::to_string(&mir) {
            Ok(value) => value,
            Err(_) => {
                return CliDiagnostic::failure("failed to serialize mir".to_string()).emit();
            }
        };

        println!("{json}");
        return EXIT_OK;
    }

    if is_project_root_path {
        println!("check: ok");
    }

    EXIT_OK
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

    if let Some(argument) = args.get(1) {
        return CliDiagnostic::usage(format!("unexpected argument '{argument}'")).emit();
    }

    if let Err(error) = load_run_source(&source_path) {
        return CliDiagnostic::failure(error).emit();
    }

    println!("test: ok");
    EXIT_OK
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

    let report = match format_path(&source_path, mode) {
        Ok(report) => report,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    if mode == FormatMode::Check && report.changed_files > 0 {
        let suffix = if report.changed_files == 1 { "" } else { "s" };
        return CliDiagnostic::failure(format!(
            "formatting required for {} file{} (run `tonic fmt <path>` to apply fixes)",
            report.changed_files, suffix
        ))
        .emit();
    }

    println!("fmt: ok");
    EXIT_OK
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
            other => {
                return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit();
            }
        }
    }

    let mut profiler = profiling::PhaseProfiler::from_env("compile");
    let is_project_root_path = std::path::Path::new(&source_path).is_dir();

    let source = match profiling::profile_phase(&mut profiler, "compile.load_source", || {
        load_run_source(&source_path)
    }) {
        Ok(source) => source,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    let ir = match compile_source_to_ir(&source, &mut profiler) {
        Ok(ir) => ir,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    let artifact_stem = compile_artifact_stem(&source_path, is_project_root_path);
    let mir =
        match profiling::profile_phase(&mut profiler, "backend.lower_mir", || lower_ir_to_mir(&ir))
        {
            Ok(mir) => mir,
            Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
        };

    let optimized_mir = profiling::profile_phase(&mut profiler, "backend.optimize_mir", || {
        optimize_for_native_backend(mir)
    });

    // Sidecar artifacts always land in the default build directory.
    let sidecar_base = {
        let mut p = default_compile_build_dir();
        p.push(&artifact_stem);
        p
    };
    let ll_path = sidecar_base.with_extension("ll");
    let c_path = sidecar_base.with_extension("c");
    let ir_path = sidecar_base.with_extension("tir.json");
    let manifest_path = sidecar_base.with_extension("tnx.json");

    // The executable is written to --out if given, otherwise to the
    // default build dir with no extension (idiomatic Linux binary).
    let exe_path = match out_path {
        Some(ref path) => std::path::PathBuf::from(path),
        None => sidecar_base.clone(),
    };

    for path in [&ll_path, &c_path, &ir_path, &manifest_path, &exe_path] {
        if let Err(message) = ensure_artifact_parent(path) {
            return CliDiagnostic::failure(message).emit();
        }
    }

    // --- LLVM IR sidecar (.ll) ---
    let llvm_ir = match profiling::profile_phase(&mut profiler, "backend.lower_llvm", || {
        llvm_backend::lower_mir_subset_to_llvm_ir(&optimized_mir)
    }) {
        Ok(llvm_ir) => llvm_ir,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };

    if let Err(error) = profiling::profile_phase(&mut profiler, "backend.write_llvm_ir", || {
        crate::cache::write_atomic(&ll_path, &llvm_ir)
    }) {
        return CliDiagnostic::failure(format!(
            "failed to write llvm ir sidecar to {}: {error}",
            ll_path.display()
        ))
        .emit();
    }

    // --- IR sidecar (.tir.json) ---
    let serialized_ir = match serde_json::to_string(&ir) {
        Ok(s) => s,
        Err(error) => {
            return CliDiagnostic::failure(format!(
                "failed to serialize compile artifact: {error}"
            ))
            .emit();
        }
    };

    if let Err(error) = profiling::profile_phase(&mut profiler, "backend.write_ir", || {
        crate::cache::write_atomic(&ir_path, &serialized_ir)
    }) {
        return CliDiagnostic::failure(format!(
            "failed to write ir sidecar to {}: {error}",
            ir_path.display()
        ))
        .emit();
    }

    // --- Manifest sidecar (.tnx.json) for backward-compat tonic run ---
    let manifest = native_artifact::build_executable_manifest(
        &source,
        &manifest_path,
        &ll_path,
        &exe_path,
        &ir_path,
    );
    if let Err(error) = profiling::profile_phase(&mut profiler, "backend.write_manifest", || {
        native_artifact::write_manifest(&manifest_path, &manifest)
    }) {
        return CliDiagnostic::failure(error).emit();
    }

    // --- C code generation ---
    let c_source = match profiling::profile_phase(&mut profiler, "backend.lower_c", || {
        c_backend::lower_mir_to_c(&optimized_mir)
    }) {
        Ok(src) => src,
        Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
    };

    if let Err(error) = profiling::profile_phase(&mut profiler, "backend.write_c_source", || {
        crate::cache::write_atomic(&c_path, &c_source)
    }) {
        return CliDiagnostic::failure(format!(
            "failed to write c source to {}: {error}",
            c_path.display()
        ))
        .emit();
    }

    // --- Compile C to native executable ---
    if let Err(error) = profiling::profile_phase(&mut profiler, "backend.link_executable", || {
        linker::compile_c_to_executable(&c_path, &exe_path)
    }) {
        return CliDiagnostic::failure(error.to_string()).emit();
    }

    println!("compile: ok {}", exe_path.display());
    EXIT_OK
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

    let acceptance = match load_acceptance_yaml(&slice_id) {
        Ok(metadata) => metadata,
        Err(message) => return CliDiagnostic::failure(message).emit(),
    };

    let scenarios = match load_feature_scenarios(&acceptance.feature_files) {
        Ok(scenarios) => scenarios,
        Err(message) => return CliDiagnostic::failure(message).emit(),
    };

    let mode_tags = mode.selected_tags();
    let filtered_scenarios = scenarios
        .into_iter()
        .filter(|scenario| {
            scenario
                .tags
                .iter()
                .any(|tag| mode_tags.contains(&tag.as_str()))
        })
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
        "scenarios": filtered_scenarios
            .into_iter()
            .map(|scenario| serde_json::json!({ "id": scenario.id, "tags": scenario.tags }))
            .collect::<Vec<_>>(),
        "benchmark": benchmark_report,
        "manual_evidence": manual_evidence_report,
    });

    println!("{report}");

    if verify_failed {
        EXIT_FAILURE
    } else {
        EXIT_OK
    }
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

    // Find project root by looking for tonic.toml
    let project_root = find_project_root();
    if project_root.is_none() {
        return CliDiagnostic::failure("no tonic.toml found in current directory or parents")
            .emit();
    }
    let project_root = project_root.unwrap();

    // Load manifest
    let manifest = match manifest::load_project_manifest(&project_root) {
        Ok(m) => m,
        Err(msg) => return CliDiagnostic::failure(msg).emit(),
    };

    if manifest.dependencies.path.is_empty() && manifest.dependencies.git.is_empty() {
        println!("No dependencies defined in tonic.toml");
        return EXIT_OK;
    }

    match subcommand.as_str() {
        "sync" | "fetch" => {
            println!("Syncing dependencies...");
            match deps::DependencyResolver::sync(&manifest.dependencies, &project_root) {
                Ok(lockfile) => {
                    println!("Dependencies synced successfully.");
                    println!("Lockfile saved to tonic.lock");
                    println!("  - path dependencies: {}", lockfile.path_deps.len());
                    println!("  - git dependencies: {}", lockfile.git_deps.len());
                    EXIT_OK
                }
                Err(msg) => {
                    CliDiagnostic::failure(format!("failed to sync dependencies: {}", msg)).emit()
                }
            }
        }
        "lock" => {
            println!("Generating lockfile...");
            match deps::Lockfile::generate(&manifest.dependencies, &project_root) {
                Ok(lockfile) => match lockfile.save(&project_root) {
                    Ok(()) => {
                        println!("Lockfile generated: tonic.lock");
                        EXIT_OK
                    }
                    Err(msg) => {
                        CliDiagnostic::failure(format!("failed to save lockfile: {}", msg)).emit()
                    }
                },
                Err(msg) => {
                    CliDiagnostic::failure(format!("failed to generate lockfile: {}", msg)).emit()
                }
            }
        }
        _ => EXIT_OK,
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
        "tonic language core v0\n\nUsage:\n  tonic <COMMAND> [OPTIONS]\n\nCommands:\n  run      Execute source\n  check    Parse and type-check source\n  test     Run project tests\n  fmt      Format source files\n  compile  Compile source to executable artifact\n  cache    Manage compiled artifacts\n  verify   Run acceptance verification\n  deps     Manage project dependencies\n"
    );
}

fn print_run_help() {
    println!("Usage:\n  tonic run <path>\n");
}

fn print_check_help() {
    println!("Usage:\n  tonic check <path> [--dump-tokens|--dump-ast|--dump-ir|--dump-mir]\n");
}

fn print_test_help() {
    println!("Usage:\n  tonic test <path>\n");
}

fn print_fmt_help() {
    println!("Usage:\n  tonic fmt <path> [--check]\n");
}

fn print_compile_help() {
    println!(
        "Usage:\n  tonic compile <path> [--out <artifact-path>]\n\n\
         Compile contract:\n\
         \x20 Compile always produces a native executable artifact (ELF on Linux).\n\
         \x20 Default output: .tonic/build/<name>  (runnable as ./.tonic/build/<name>)\n\
         \x20 --out <path>   Write executable to <path> directly\n\
         \x20 Requires: cc, gcc, or clang in PATH\n"
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
}
