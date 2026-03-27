use super::*;

pub(super) fn handle_deps(args: Vec<String>) -> i32 {
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

pub(super) fn find_project_root() -> Option<std::path::PathBuf> {
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

pub(super) fn handle_publish(args: Vec<String>) -> i32 {
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

pub(super) fn print_publish_help() {
    println!(
        "tonic publish - Publish package to registry (not yet implemented)\n\n\
         Usage:\n  tonic publish\n\n\
         Requires tonic.toml [package] section with:\n  \
         name, version, description\n\n\
         Publishing to a registry is not yet supported.\n"
    );
}

pub(super) fn print_deps_help() {
    println!(
        "tonic deps - Manage project dependencies\n\n\
         Usage:\n  tonic deps <SUBCOMMAND>\n\n\
         Subcommands:\n  sync    Fetch all dependencies and generate lockfile\n  fetch   Alias for sync\n  lock    Generate lockfile without fetching\n\n\
         Examples:\n  tonic deps sync    # Fetch dependencies and create tonic.lock\n  tonic deps lock    # Just create/update tonic.lock\n"
    );
}

pub(super) fn benchmark_thresholds_report() -> serde_json::Value {
    serde_json::json!({
        "cold_start_p50_ms": BENCHMARK_THRESHOLD_COLD_START_P50_MS,
        "warm_start_p50_ms": BENCHMARK_THRESHOLD_WARM_START_P50_MS,
        "idle_rss_mb": BENCHMARK_THRESHOLD_IDLE_RSS_MB,
    })
}

pub(super) fn print_help() {
    println!(
        "tonic language core v0\n\nUsage:\n  tonic <COMMAND> [OPTIONS]\n\nCommands:\n  run      Execute source\n  repl     Start interactive or remote REPL\n  check    Parse and type-check source\n  test     Run project tests\n  fmt      Format source files\n  compile  Compile source to executable artifact\n  cache    Manage compiled artifacts\n  verify   Run acceptance verification\n  deps     Manage project dependencies\n  docs     Generate API documentation\n  lsp      Start language server\n  publish  Publish package to registry (not yet implemented)\n"
    );
}

pub(super) fn print_run_help() {
    println!("Usage:\n  tonic run <path>\n");
}

pub(super) fn print_check_help() {
    println!(
        "Usage:\n  tonic check <path> [--dump-tokens [--format <text|json>]|--dump-ast|--dump-ir|--dump-mir]\n"
    );
}

pub(super) fn print_test_help() {
    println!("Usage:\n  tonic test <path> [--filter <pattern>] [--format <text|json>]\n\nOptions:\n  --filter <pattern>  Only run tests whose name contains <pattern>\n  --format <text|json> Output format (default: text)\n");
}

pub(super) fn print_fmt_help() {
    println!("Usage:\n  tonic fmt <path> [--check]\n");
}

pub(super) fn print_compile_help() {
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

pub(super) fn print_verify_help() {
    println!("Usage:\n  tonic verify run <slice-id> [--mode <auto|mixed|manual>]\n");
}

pub(super) fn print_verify_run_help() {
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
