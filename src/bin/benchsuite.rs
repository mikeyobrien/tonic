#[path = "benchsuite/competitive.rs"]
mod competitive;
#[path = "benchsuite/competitive_slo.rs"]
mod competitive_slo;
#[cfg(test)]
#[path = "benchsuite/competitive_tests.rs"]
mod competitive_tests;
#[path = "benchsuite/model.rs"]
mod model;
#[path = "benchsuite/runtime.rs"]
mod runtime;
#[path = "benchsuite/utils.rs"]
mod utils;

use competitive::evaluate_contract;
use model::{help_text, parse_cli_args, SuiteManifest, SuiteReport, WorkloadReport};
use runtime::{
    calculate_calibrated_threshold, collect_host_metadata, evaluate_thresholds, run_workload,
    workload_report_from_error, write_json_report, write_markdown_report,
};
use std::env;
use std::fs;

fn main() {
    let args = match parse_cli_args(env::args().skip(1)) {
        Ok(args) => args,
        Err(message) if message == "__PRINT_HELP__" => {
            print!("{}", help_text());
            std::process::exit(0);
        }
        Err(message) => {
            eprintln!("usage error: {message}");
            eprintln!("run `benchsuite --help` for usage");
            std::process::exit(2);
        }
    };

    if !args.manifest_path.is_file() {
        eprintln!(
            "benchmark manifest not found at {}",
            args.manifest_path.display()
        );
        std::process::exit(1);
    }

    if !args.bin_path.is_file() {
        eprintln!("tonic binary not found at {}", args.bin_path.display());
        eprintln!("hint: build it first with `cargo build --release`");
        std::process::exit(1);
    }

    let manifest_str = match fs::read_to_string(&args.manifest_path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!(
                "failed to read benchmark manifest {}: {error}",
                args.manifest_path.display()
            );
            std::process::exit(1);
        }
    };

    let suite: SuiteManifest = match toml::from_str(&manifest_str) {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!(
                "failed to parse benchmark manifest {}: {error}",
                args.manifest_path.display()
            );
            std::process::exit(1);
        }
    };

    if suite.workload.is_empty() {
        eprintln!(
            "benchmark manifest {} has no workloads",
            args.manifest_path.display()
        );
        std::process::exit(1);
    }

    let metadata = collect_host_metadata();
    let mut reports: Vec<WorkloadReport> = Vec::new();
    let mut has_failures = false;

    for workload in &suite.workload {
        println!("running workload: {} ({})", workload.name, workload.mode);

        let report = match run_workload(&args.bin_path, workload, args.runs, args.warmup_runs) {
            Ok(stats) => {
                let (p50_exceeded, p95_exceeded, rss_exceeded) =
                    evaluate_thresholds(&stats, workload);
                if !args.calibrate && (p50_exceeded || p95_exceeded || rss_exceeded == Some(true)) {
                    has_failures = true;
                }

                let (suggested_p50, suggested_p95) = if args.calibrate {
                    (
                        Some(calculate_calibrated_threshold(
                            stats.p50_ms,
                            args.calibrate_margin_pct,
                        )),
                        Some(calculate_calibrated_threshold(
                            stats.p95_ms,
                            args.calibrate_margin_pct,
                        )),
                    )
                } else {
                    (None, None)
                };

                WorkloadReport {
                    name: workload.name.clone(),
                    command: workload.command.clone(),
                    mode: workload.mode.clone(),
                    status: if args.calibrate {
                        "calibrated".to_string()
                    } else if p50_exceeded || p95_exceeded || rss_exceeded == Some(true) {
                        "threshold_exceeded".to_string()
                    } else {
                        "pass".to_string()
                    },
                    threshold_p50_ms: workload.threshold_p50_ms,
                    threshold_p95_ms: workload.threshold_p95_ms,
                    threshold_rss_kb: workload.threshold_rss_kb,
                    category: workload.category.clone(),
                    weight: Some(workload.weight),
                    p50_ms: Some(stats.p50_ms),
                    p95_ms: Some(stats.p95_ms),
                    p50_exceeded: if args.calibrate { false } else { p50_exceeded },
                    p95_exceeded: if args.calibrate { false } else { p95_exceeded },
                    rss_exceeded: if args.calibrate {
                        Some(false)
                    } else {
                        rss_exceeded
                    },
                    suggested_threshold_p50_ms: suggested_p50,
                    suggested_threshold_p95_ms: suggested_p95,
                    peak_rss_kb: stats.peak_rss_kb,
                    error: None,
                    samples_ms: Some(stats.samples_ms),
                }
            }
            Err(error) => {
                has_failures = true;
                workload_report_from_error(workload, error)
            }
        };

        if let Some(p50) = report.p50_ms {
            println!("  p50={p50:.2}ms (threshold {}ms)", report.threshold_p50_ms);
        }
        if let Some(p95) = report.p95_ms {
            println!("  p95={p95:.2}ms (threshold {}ms)", report.threshold_p95_ms);
        }
        if let Some(rss) = report.peak_rss_kb {
            if let Some(threshold) = report.threshold_rss_kb {
                println!("  rss={rss} KB (threshold {threshold}KB)");
            } else {
                println!("  rss={rss} KB");
            }
        }
        if let (Some(s_p50), Some(s_p95)) = (
            report.suggested_threshold_p50_ms,
            report.suggested_threshold_p95_ms,
        ) {
            println!("  suggested: p50={s_p50}ms, p95={s_p95}ms");
        }
        if let Some(error) = &report.error {
            println!("  error={error}");
        }

        reports.push(report);
    }

    let contract_report = if args.calibrate {
        None
    } else {
        match evaluate_contract(&suite, &reports, &args) {
            Ok(result) => result,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    };

    if let Some(contract_report) = &contract_report {
        if contract_report.status == "fail" {
            has_failures = true;
        }
    }

    let report = SuiteReport {
        suite_path: args.manifest_path.display().to_string(),
        bin_path: args.bin_path.display().to_string(),
        runs: args.runs,
        warmup_runs: args.warmup_runs,
        status: if has_failures {
            "fail".to_string()
        } else if args.calibrate {
            "calibrated".to_string()
        } else {
            "pass".to_string()
        },
        workloads: reports,
        metadata,
        performance_contract: contract_report,
    };

    if let Err(error) = write_json_report(&args.json_out, &report) {
        eprintln!("{error}");
        std::process::exit(1);
    }

    println!("wrote json report: {}", args.json_out.display());

    if let Some(markdown_path) = &args.markdown_out {
        if let Err(error) = write_markdown_report(markdown_path, &report) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        println!("wrote markdown report: {}", markdown_path.display());
    }

    let stdout_payload =
        serde_json::to_string_pretty(&report).expect("suite report should serialize to json");
    println!("{stdout_payload}");

    if args.enforce && has_failures {
        eprintln!("benchmark suite failed in enforce mode");
        std::process::exit(1);
    }
}
