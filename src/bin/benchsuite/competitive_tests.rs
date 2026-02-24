#[cfg(test)]
mod tests {
    use crate::competitive::evaluate_with_bundle;
    use crate::model::{
        BaselineBundle, BaselineTarget, BaselineWorkload, CliArgs, HostMetadata, MetricWeights,
        NativeSlo, PerformanceContract, SuiteManifest, Workload, WorkloadReport,
    };
    use std::path::PathBuf;

    #[test]
    fn evaluate_contract_flags_relative_regression_and_low_score() {
        let contract = PerformanceContract {
            baseline_path: "unused.json".to_string(),
            candidate_target: "interpreter".to_string(),
            reference_targets: vec!["rust".to_string(), "go".to_string()],
            relative_budget_pct: 20.0,
            pass_threshold: 0.9,
            metric_weights: MetricWeights {
                latency_p50: 0.4,
                latency_p95: 0.3,
                rss: 0.2,
                artifact_size: 0.05,
                compile_latency: 0.05,
            },
            slo: NativeSlo {
                startup_p50_ms: Some(50),
                runtime_p50_ms: Some(20),
                runtime_p95_ms: Some(30),
                rss_kb: Some(30_000),
                artifact_size_bytes: Some(10_000_000),
                compile_latency_ms: Some(2_000),
            },
        };

        let manifest = SuiteManifest {
            performance_contract: Some(contract.clone()),
            workload: vec![Workload {
                name: "run_pipeline".to_string(),
                command: vec!["run".to_string(), "fixtures/pipeline.tn".to_string()],
                mode: "cold".to_string(),
                threshold_p50_ms: 100,
                threshold_p95_ms: 120,
                threshold_rss_kb: Some(30_000),
                weight: 1.0,
                category: Some("startup".to_string()),
            }],
        };

        let reports = vec![WorkloadReport {
            name: "run_pipeline".to_string(),
            command: vec![],
            mode: "cold".to_string(),
            status: "pass".to_string(),
            threshold_p50_ms: 100,
            threshold_p95_ms: 120,
            threshold_rss_kb: Some(30_000),
            category: Some("startup".to_string()),
            weight: Some(1.0),
            p50_ms: Some(90.0),
            p95_ms: Some(110.0),
            p50_exceeded: false,
            p95_exceeded: false,
            rss_exceeded: Some(false),
            suggested_threshold_p50_ms: None,
            suggested_threshold_p95_ms: None,
            peak_rss_kb: Some(29_000),
            error: None,
            samples_ms: None,
        }];

        let baseline = BaselineBundle {
            metadata: Some(HostMetadata {
                captured_at_utc: Some("2026-02-24T00:00:00Z".to_string()),
                os: "linux".to_string(),
                arch: "x86_64".to_string(),
                kernel: Some("6.12".to_string()),
                cpu_model: Some("x".to_string()),
                rustc_version: Some("rustc 1.86.0".to_string()),
                go_version: Some("go1.25".to_string()),
            }),
            targets: vec![
                BaselineTarget {
                    name: "rust".to_string(),
                    artifact_size_bytes: Some(6_000_000),
                    compile_latency_ms: Some(900),
                    workloads: vec![BaselineWorkload {
                        name: "run_pipeline".to_string(),
                        p50_ms: 50.0,
                        p95_ms: 70.0,
                        peak_rss_kb: Some(12_000),
                    }],
                },
                BaselineTarget {
                    name: "go".to_string(),
                    artifact_size_bytes: Some(7_000_000),
                    compile_latency_ms: Some(600),
                    workloads: vec![BaselineWorkload {
                        name: "run_pipeline".to_string(),
                        p50_ms: 55.0,
                        p95_ms: 65.0,
                        peak_rss_kb: Some(14_000),
                    }],
                },
            ],
        };

        let args = CliArgs {
            bin_path: PathBuf::from("target/release/tonic"),
            manifest_path: PathBuf::from("benchmarks/suite.toml"),
            runs: 10,
            warmup_runs: 2,
            enforce: true,
            calibrate: false,
            calibrate_margin_pct: 20,
            json_out: PathBuf::from("benchmarks/summary.json"),
            markdown_out: None,
            compile_latency_ms: Some(2_500),
            target_name: "interpreter".to_string(),
        };

        let result = evaluate_with_bundle(&contract, &baseline, &manifest, &reports, &args);
        assert_eq!(result.status, "fail");
        assert!(result
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("relative budget")));
        assert!(result
            .failure_reasons
            .iter()
            .any(|reason| reason.contains("overall score")));
    }

    #[test]
    fn evaluate_contract_requires_compile_latency_when_weighted() {
        let contract = PerformanceContract {
            baseline_path: "unused.json".to_string(),
            candidate_target: "interpreter".to_string(),
            reference_targets: vec!["rust".to_string(), "go".to_string()],
            relative_budget_pct: 25.0,
            pass_threshold: 0.1,
            metric_weights: MetricWeights {
                latency_p50: 0.4,
                latency_p95: 0.3,
                rss: 0.2,
                artifact_size: 0.0,
                compile_latency: 0.1,
            },
            slo: NativeSlo::default(),
        };

        let manifest = SuiteManifest {
            performance_contract: Some(contract.clone()),
            workload: vec![Workload {
                name: "run_pipeline".to_string(),
                command: vec!["run".to_string(), "fixtures/pipeline.tn".to_string()],
                mode: "warm".to_string(),
                threshold_p50_ms: 100,
                threshold_p95_ms: 120,
                threshold_rss_kb: Some(30_000),
                weight: 1.0,
                category: Some("runtime".to_string()),
            }],
        };

        let reports = vec![WorkloadReport {
            name: "run_pipeline".to_string(),
            command: vec![],
            mode: "warm".to_string(),
            status: "pass".to_string(),
            threshold_p50_ms: 100,
            threshold_p95_ms: 120,
            threshold_rss_kb: Some(30_000),
            category: Some("runtime".to_string()),
            weight: Some(1.0),
            p50_ms: Some(20.0),
            p95_ms: Some(25.0),
            p50_exceeded: false,
            p95_exceeded: false,
            rss_exceeded: Some(false),
            suggested_threshold_p50_ms: None,
            suggested_threshold_p95_ms: None,
            peak_rss_kb: Some(10_000),
            error: None,
            samples_ms: None,
        }];

        let baseline = BaselineBundle {
            metadata: None,
            targets: vec![
                BaselineTarget {
                    name: "rust".to_string(),
                    artifact_size_bytes: Some(6_000_000),
                    compile_latency_ms: Some(900),
                    workloads: vec![BaselineWorkload {
                        name: "run_pipeline".to_string(),
                        p50_ms: 18.0,
                        p95_ms: 22.0,
                        peak_rss_kb: Some(8_000),
                    }],
                },
                BaselineTarget {
                    name: "go".to_string(),
                    artifact_size_bytes: Some(7_000_000),
                    compile_latency_ms: Some(700),
                    workloads: vec![BaselineWorkload {
                        name: "run_pipeline".to_string(),
                        p50_ms: 19.0,
                        p95_ms: 23.0,
                        peak_rss_kb: Some(8_500),
                    }],
                },
            ],
        };

        let args = CliArgs {
            bin_path: PathBuf::from("target/release/tonic"),
            manifest_path: PathBuf::from("benchmarks/suite.toml"),
            runs: 10,
            warmup_runs: 2,
            enforce: true,
            calibrate: false,
            calibrate_margin_pct: 20,
            json_out: PathBuf::from("benchmarks/summary.json"),
            markdown_out: None,
            compile_latency_ms: None,
            target_name: "interpreter".to_string(),
        };

        let result = evaluate_with_bundle(&contract, &baseline, &manifest, &reports, &args);
        assert_eq!(result.status, "fail");
        assert!(result
            .failure_reasons
            .iter()
            .any(|reason| reason == "missing --compile-latency-ms value for performance contract"));
    }
}
