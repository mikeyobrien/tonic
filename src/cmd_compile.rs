use super::*;

pub(super) fn handle_compile(args: Vec<String>) -> i32 {
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

pub(super) fn compile_artifact_stem(source_path: &str, is_project_root_path: bool) -> String {
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

pub(super) fn default_compile_build_dir() -> std::path::PathBuf {
    let mut path = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    path.push(".tonic");
    path.push("build");
    path
}

pub(super) fn ensure_artifact_parent(path: &std::path::Path) -> Result<(), String> {
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
