use super::*;
use crate::interop::{host_stdout_was_observed, reset_host_stdout_observed};

pub(super) fn handle_run(args: Vec<String>) -> i32 {
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

    reset_host_stdout_observed();
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
            if !host_stdout_was_observed() {
                println!("{}", other.render());
            }
            finalize_observed_run(&mut observed_run, EXIT_OK, None)
        }
    }
}

pub(super) fn handle_run_native_artifact(
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

    reset_host_stdout_observed();
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
            if !host_stdout_was_observed() {
                println!("{}", other.render());
            }
            finalize_observed_run(observed_run, EXIT_OK, None)
        }
    }
}

pub(super) fn compile_source_to_ir(
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

pub(super) fn maybe_trace_type_summary(signature_count: usize) {
    if std::env::var_os("TONIC_DEBUG_TYPES").is_some() {
        eprintln!("type-signatures {signature_count}");
    }
}

pub(super) fn run_placeholder(command: &str) -> i32 {
    println!("tonic {command} command skeleton");
    EXIT_OK
}
