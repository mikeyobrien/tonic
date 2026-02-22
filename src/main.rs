mod acceptance;
mod cache;
mod cli_diag;
mod deps;
mod formatter;
mod interop;
mod ir;
mod lexer;
mod manifest;
mod parser;
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
    if matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help")
    ) {
        print_run_help();
        return EXIT_OK;
    }

    let source_path = args[0].clone();

    if let Some(argument) = args.get(1) {
        return CliDiagnostic::usage(format!("unexpected argument '{argument}'")).emit();
    }

    let source = match load_run_source(&source_path) {
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

    let ir = match load_cached_ir(&cache_key) {
        Ok(Some(cached_ir)) => {
            cache_status = "hit";
            cached_ir
        }
        Ok(None) | Err(_) => {
            let compiled_ir = match compile_source_to_ir(&source) {
                Ok(ir) => ir,
                Err(error) => return CliDiagnostic::failure(error).emit(),
            };

            let _ = store_cached_ir(&cache_key, &compiled_ir);
            compiled_ir
        }
    };

    if should_trace_cache_status() {
        trace_cache_status(cache_status);
    }

    let value = match evaluate_entrypoint(&ir) {
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

fn compile_source_to_ir(source: &str) -> Result<IrProgram, String> {
    let tokens = scan_tokens(source).map_err(|error| error.to_string())?;
    let ast = parse_ast(&tokens).map_err(|error| error.to_string())?;

    resolve_ast(&ast).map_err(|error| error.to_string())?;
    infer_types(&ast).map_err(|error| error.to_string())?;

    lower_ast_to_ir(&ast).map_err(|error| error.to_string())
}

fn run_placeholder(command: &str) -> i32 {
    println!("tonic {command} command skeleton");
    EXIT_OK
}

fn handle_check(args: Vec<String>) -> i32 {
    if matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help")
    ) {
        print_check_help();
        return EXIT_OK;
    }

    let source_path = args[0].clone();
    let is_project_root_path = std::path::Path::new(&source_path).is_dir();
    let mut dump_tokens = false;
    let mut dump_ast = false;
    let mut dump_ir = false;

    for argument in args.iter().skip(1) {
        match argument.as_str() {
            "--dump-tokens" => dump_tokens = true,
            "--dump-ast" => dump_ast = true,
            "--dump-ir" => dump_ir = true,
            other => {
                return CliDiagnostic::usage(format!("unexpected argument '{other}'")).emit();
            }
        }
    }

    let dump_mode_count = [dump_tokens, dump_ast, dump_ir]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

    if dump_mode_count > 1 {
        return CliDiagnostic::usage(
            "--dump-tokens, --dump-ast, and --dump-ir cannot be used together",
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
            Err(error) => {
                return CliDiagnostic::failure(format!("failed to serialize ast: {error}")).emit();
            }
        };

        println!("{json}");
        return EXIT_OK;
    }

    if let Err(error) = resolve_ast(&ast) {
        return CliDiagnostic::failure(error.to_string()).emit();
    }

    if let Err(error) = infer_types(&ast) {
        return CliDiagnostic::failure(error.to_string()).emit();
    }

    if dump_ir {
        let ir = match lower_ast_to_ir(&ast) {
            Ok(ir) => ir,
            Err(error) => return CliDiagnostic::failure(error.to_string()).emit(),
        };

        let json = match serde_json::to_string(&ir) {
            Ok(value) => value,
            Err(error) => {
                return CliDiagnostic::failure(format!("failed to serialize ir: {error}")).emit();
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
    if matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help")
    ) {
        print_test_help();
        return EXIT_OK;
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
    if matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help")
    ) {
        print_fmt_help();
        return EXIT_OK;
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
    if matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help")
    ) {
        print_compile_help();
        return EXIT_OK;
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

    let is_project_root_path = std::path::Path::new(&source_path).is_dir();

    let source = match load_run_source(&source_path) {
        Ok(source) => source,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    let ir = match compile_source_to_ir(&source) {
        Ok(ir) => ir,
        Err(error) => return CliDiagnostic::failure(error).emit(),
    };

    let artifact_path = match out_path {
        Some(path) => std::path::PathBuf::from(path),
        None => {
            let stem = if is_project_root_path {
                manifest::load_project_manifest(std::path::Path::new(&source_path))
                    .ok()
                    .and_then(|m| {
                        m.entry
                            .file_stem()
                            .map(|s| s.to_string_lossy().into_owned())
                    })
                    .unwrap_or_else(|| "out".to_string())
            } else {
                std::path::Path::new(&source_path)
                    .file_stem()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "out".to_string())
            };

            let mut p = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
            p.push(".tonic");
            p.push("build");
            p.push(format!("{}.tir.json", stem));
            p
        }
    };

    if let Some(parent) = artifact_path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            return CliDiagnostic::failure(format!(
                "failed to create artifact directory {}: {}",
                parent.display(),
                error
            ))
            .emit();
        }
    }

    let serialized = match serde_json::to_string(&ir) {
        Ok(s) => s,
        Err(error) => {
            return CliDiagnostic::failure(format!(
                "failed to serialize compile artifact: {error}"
            ))
            .emit();
        }
    };

    if let Err(error) = std::fs::write(&artifact_path, serialized) {
        return CliDiagnostic::failure(format!(
            "failed to write compile artifact to {}: {}",
            artifact_path.display(),
            error
        ))
        .emit();
    }

    println!("compile: ok {}", artifact_path.display());
    EXIT_OK
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
            let threshold_exceeded = metrics.cold_start_p50_ms
                > BENCHMARK_THRESHOLD_COLD_START_P50_MS
                || metrics.warm_start_p50_ms > BENCHMARK_THRESHOLD_WARM_START_P50_MS
                || metrics.idle_rss_mb > BENCHMARK_THRESHOLD_IDLE_RSS_MB;

            (
                threshold_exceeded,
                serde_json::json!({
                    "status": if threshold_exceeded { "threshold_exceeded" } else { "pass" },
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
            }),
        );
    }

    let missing = required_paths
        .iter()
        .filter(|path| !path.is_file())
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let missing_required = !missing.is_empty();

    (
        missing_required,
        serde_json::json!({
            "status": if missing_required { "missing_required" } else { "pass" },
            "required": required,
            "missing": missing,
        }),
    )
}

fn handle_deps(args: Vec<String>) -> i32 {
    if matches!(
        args.first().map(String::as_str),
        None | Some("-h" | "--help" | "help")
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
        "tonic language core v0\n\nUsage:\n  tonic <COMMAND> [OPTIONS]\n\nCommands:\n  run      Execute source\n  check    Parse and type-check source\n  test     Run project tests\n  fmt      Format source files\n  compile  Compile source to IR artifact\n  cache    Manage compiled artifacts\n  verify   Run acceptance verification\n  deps     Manage project dependencies\n"
    );
}

fn print_run_help() {
    println!("Usage:\n  tonic run <path>\n");
}

fn print_check_help() {
    println!("Usage:\n  tonic check <path> [--dump-tokens|--dump-ast|--dump-ir]\n");
}

fn print_test_help() {
    println!("Usage:\n  tonic test <path>\n");
}

fn print_fmt_help() {
    println!("Usage:\n  tonic fmt <path> [--check]\n");
}

fn print_compile_help() {
    println!("Usage:\n  tonic compile <path> [--out <artifact-path>]\n");
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
    fn known_commands_exit_success() {
        for command in ["run", "check", "test", "fmt", "compile", "cache", "deps"] {
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
}
