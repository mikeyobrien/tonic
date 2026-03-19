use super::*;

pub(super) fn handle_verify(args: Vec<String>) -> i32 {
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

pub(super) fn handle_verify_run(args: Vec<String>) -> i32 {
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

pub(super) fn benchmark_gate_report(
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

pub(super) fn manual_evidence_gate_report(
    required_paths: &[std::path::PathBuf],
) -> (bool, serde_json::Value) {
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
