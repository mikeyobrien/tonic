use super::*;

pub(super) fn handle_check(args: Vec<String>) -> i32 {
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
                    return CliDiagnostic::usage_with_hint(
                        "missing value for --format",
                        "usage: tonic check <path> --format <text|json>",
                    )
                    .emit();
                };

                let Some(parsed) = TestOutputFormat::parse(value) else {
                    return CliDiagnostic::usage_with_hint(
                        format!("unsupported format '{value}' (expected 'text' or 'json')"),
                        "valid formats: text, json",
                    )
                    .emit();
                };

                token_dump_format = parsed;
                token_dump_format_explicit = true;
                index += 2;
            }
            other => {
                return CliDiagnostic::usage_with_hint(
                    format!("unexpected argument '{other}'"),
                    "run `tonic check --help` for usage",
                )
                .emit();
            }
        }
    }

    let dump_mode_count = [dump_tokens, dump_ast, dump_ir, dump_mir]
        .into_iter()
        .filter(|enabled| *enabled)
        .count();

    if dump_mode_count > 1 {
        return CliDiagnostic::usage_with_hint(
            "--dump-tokens, --dump-ast, --dump-ir, and --dump-mir cannot be used together",
            "use only one dump flag at a time",
        )
        .emit();
    }

    if token_dump_format_explicit && !dump_tokens {
        return CliDiagnostic::usage_with_hint(
            "--format is only supported with --dump-tokens",
            "add `--dump-tokens` to use `--format`",
        )
        .emit();
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
