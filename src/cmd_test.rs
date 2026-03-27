use super::*;

pub(super) fn handle_test(args: Vec<String>) -> i32 {
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
    let mut filter: Option<String> = None;
    let mut list_only = false;
    let mut fail_fast = false;
    let mut index = 1;

    while index < args.len() {
        match args[index].as_str() {
            "--format" => {
                let Some(value) = args.get(index + 1) else {
                    return CliDiagnostic::usage_with_hint(
                        "missing value for --format",
                        "usage: tonic test <path> --format <text|json>",
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

                format = parsed;
                index += 2;
            }
            "--list" => {
                list_only = true;
                index += 1;
            }
            "--fail-fast" => {
                fail_fast = true;
                index += 1;
            }
            "--filter" => {
                let Some(value) = args.get(index + 1) else {
                    return CliDiagnostic::usage_with_hint(
                        "missing value for --filter",
                        "usage: tonic test <path> --filter <pattern>",
                    )
                    .emit();
                };

                filter = Some(value.clone());
                index += 2;
            }
            other => {
                return CliDiagnostic::usage_with_hint(
                    format!("unexpected argument '{other}'"),
                    "run `tonic test --help` for usage",
                )
                .emit();
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
        if let Some(ref f) = filter {
            observed_run.record_metadata("filter", f.clone());
        }
    }

    if list_only {
        let tests = match observe_command_phase_result(&mut observed_run, "test.list_tests", || {
            test_runner::list_tests(&source_path, filter.as_deref())
        }) {
            Ok(tests) => tests,
            Err(TestRunnerError::Failure(message)) => {
                let exit_code = CliDiagnostic::failure(message.clone()).emit();
                return finalize_observed_run(
                    &mut observed_run,
                    exit_code,
                    Some(make_observability_error(
                        "io_error",
                        "test.list_tests",
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
                        "test.list_tests",
                        message,
                        source_info,
                    )),
                );
            }
        };

        match format {
            TestOutputFormat::Text => {
                for name in &tests {
                    println!("{name}");
                }
            }
            TestOutputFormat::Json => {
                println!("{}", serde_json::json!({ "tests": tests }));
            }
        }

        return finalize_observed_run(&mut observed_run, EXIT_OK, None);
    }

    let report = match observe_command_phase_result(&mut observed_run, "test.run_suite", || {
        test_runner::run(&source_path, filter.as_deref(), fail_fast)
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

pub(super) fn handle_fmt(args: Vec<String>) -> i32 {
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
            other => {
                return CliDiagnostic::usage_with_hint(
                    format!("unexpected argument '{other}'"),
                    "run `tonic fmt --help` for usage",
                )
                .emit()
            }
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
