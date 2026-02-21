mod acceptance;
mod cli_diag;
mod ir;
mod lexer;
mod parser;
mod resolver;
mod resolver_diag;
mod typing;

use acceptance::{load_acceptance_yaml, load_feature_scenarios};
use cli_diag::{CliDiagnostic, EXIT_OK};
use ir::lower_ast_to_ir;
use lexer::scan_tokens;
use parser::parse_ast;
use resolver::resolve_ast;
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
        Some("run") => run_placeholder("run"),
        Some("check") => handle_check(iter.collect()),
        Some("test") => run_placeholder("test"),
        Some("fmt") => run_placeholder("fmt"),
        Some("cache") => run_placeholder("cache"),
        Some("verify") => handle_verify(iter.collect()),
        Some(other) => CliDiagnostic::usage_with_hint(
            format!("unknown command '{other}'"),
            "run `tonic --help` to see available commands",
        )
        .emit(),
    }
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

    let source = match std::fs::read_to_string(&source_path) {
        Ok(contents) => contents,
        Err(error) => {
            return CliDiagnostic::failure(format!(
                "failed to read source file {source_path}: {error}"
            ))
            .emit();
        }
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
    }

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

    let report = serde_json::json!({
        "slice_id": slice_id,
        "mode": mode.as_str(),
        "status": "pass",
        "acceptance_file": acceptance.path.display().to_string(),
        "mode_tags": mode.selected_tags(),
        "scenarios": scenarios
            .into_iter()
            .map(|scenario| serde_json::json!({ "id": scenario.id, "tags": scenario.tags }))
            .collect::<Vec<_>>(),
    });

    println!("{report}");

    EXIT_OK
}

fn print_help() {
    println!(
        "tonic language core v0\n\nUsage:\n  tonic <COMMAND> [OPTIONS]\n\nCommands:\n  run      Execute source\n  check    Parse and type-check source\n  test     Run project tests\n  fmt      Format source files\n  cache    Manage compiled artifacts\n  verify   Run acceptance verification\n"
    );
}

fn print_check_help() {
    println!("Usage:\n  tonic check <path> [--dump-tokens|--dump-ast|--dump-ir]\n");
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
        for command in ["run", "check", "test", "fmt", "cache"] {
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
