use crate::competitive_slo::evaluate_slo;
use crate::model::{
    BaselineBundle, BaselineTarget, CliArgs, CompetitiveMetrics, CompetitiveSummary,
    CompetitiveWorkloadScore, MetricWeights, NamedCompetitiveMetrics, PerformanceContract,
    SuiteManifest, Workload, WorkloadReport,
};
use crate::utils::{ratio, score_ratio};
use std::collections::HashMap;
use std::fs;

pub fn evaluate_contract(
    manifest: &SuiteManifest,
    workload_reports: &[WorkloadReport],
    args: &CliArgs,
) -> Result<Option<CompetitiveSummary>, String> {
    let Some(contract) = &manifest.performance_contract else {
        return Ok(None);
    };

    let baseline_path = &contract.baseline_path;
    let baseline_str = fs::read_to_string(baseline_path)
        .map_err(|error| format!("failed to read baseline file {baseline_path}: {error}"))?;
    let baseline_bundle: BaselineBundle = serde_json::from_str(&baseline_str)
        .map_err(|error| format!("failed to parse baseline file {baseline_path}: {error}"))?;

    Ok(Some(evaluate_with_bundle(
        contract,
        &baseline_bundle,
        manifest,
        workload_reports,
        args,
    )))
}

pub(crate) fn evaluate_with_bundle(
    contract: &PerformanceContract,
    baseline_bundle: &BaselineBundle,
    manifest: &SuiteManifest,
    workload_reports: &[WorkloadReport],
    args: &CliArgs,
) -> CompetitiveSummary {
    let budget_ratio = 1.0 + (contract.relative_budget_pct / 100.0);
    let candidate_target = if args.target_name.trim().is_empty() {
        contract.candidate_target.clone()
    } else {
        args.target_name.clone()
    };

    let workload_reports_map = workload_reports
        .iter()
        .map(|report| (report.name.as_str(), report))
        .collect::<HashMap<_, _>>();

    let mut failure_reasons = Vec::new();
    let mut workload_scores = Vec::new();
    let mut weighted_p50_sum = 0.0;
    let mut weighted_p95_sum = 0.0;
    let mut weighted_rss_sum = 0.0;
    let mut workload_weight_sum = 0.0;

    for workload in &manifest.workload {
        let Some(candidate_report) = workload_reports_map.get(workload.name.as_str()) else {
            failure_reasons.push(format!(
                "missing workload report for '{}' while evaluating performance contract",
                workload.name
            ));
            continue;
        };

        let references = contract
            .reference_targets
            .iter()
            .filter_map(|target| {
                baseline_workload_metrics(baseline_bundle, target, &workload.name).map(|metrics| {
                    NamedCompetitiveMetrics {
                        target: target.clone(),
                        metrics,
                    }
                })
            })
            .collect::<Vec<_>>();

        if references.is_empty() {
            failure_reasons.push(format!(
                "no baseline reference metrics found for workload '{}' (targets: {})",
                workload.name,
                contract.reference_targets.join(", ")
            ));
        }

        let candidate_metrics = CompetitiveMetrics {
            p50_ms: candidate_report.p50_ms,
            p95_ms: candidate_report.p95_ms,
            peak_rss_kb: candidate_report.peak_rss_kb,
        };

        let best_ref_p50 = references
            .iter()
            .filter_map(|entry| entry.metrics.p50_ms)
            .min_by(|a, b| a.total_cmp(b));
        let best_ref_p95 = references
            .iter()
            .filter_map(|entry| entry.metrics.p95_ms)
            .min_by(|a, b| a.total_cmp(b));
        let best_ref_rss = references
            .iter()
            .filter_map(|entry| entry.metrics.peak_rss_kb)
            .min();

        let p50_ratio = candidate_report
            .p50_ms
            .zip(best_ref_p50)
            .and_then(|(candidate, reference)| ratio(candidate, reference));
        let p95_ratio = candidate_report
            .p95_ms
            .zip(best_ref_p95)
            .and_then(|(candidate, reference)| ratio(candidate, reference));
        let rss_ratio = candidate_report
            .peak_rss_kb
            .zip(best_ref_rss)
            .and_then(|(candidate, reference)| ratio(candidate as f64, reference as f64));

        let p50_score = score_ratio(p50_ratio, budget_ratio).unwrap_or(0.0);
        let p95_score = score_ratio(p95_ratio, budget_ratio).unwrap_or(0.0);
        let rss_score = score_ratio(rss_ratio, budget_ratio).unwrap_or(0.0);

        let workload_metric_weight_sum = contract.metric_weights.latency_p50
            + contract.metric_weights.latency_p95
            + contract.metric_weights.rss;
        let workload_score = if workload_metric_weight_sum <= f64::EPSILON {
            1.0
        } else {
            ((p50_score * contract.metric_weights.latency_p50)
                + (p95_score * contract.metric_weights.latency_p95)
                + (rss_score * contract.metric_weights.rss))
                / workload_metric_weight_sum
        };

        let absolute_failures = absolute_workload_failures(workload, candidate_report);
        let relative_fail = p50_ratio.is_some_and(|value| value > budget_ratio)
            || p95_ratio.is_some_and(|value| value > budget_ratio)
            || rss_ratio.is_some_and(|value| value > budget_ratio);
        let workload_pass = absolute_failures.is_empty() && !relative_fail;

        if !absolute_failures.is_empty() {
            failure_reasons.extend(absolute_failures.clone());
        }
        if relative_fail {
            failure_reasons.push(format!(
                "workload '{}' exceeded relative budget {:.1}% compared to references",
                workload.name, contract.relative_budget_pct
            ));
        }

        workload_scores.push(CompetitiveWorkloadScore {
            name: workload.name.clone(),
            weight: workload.weight,
            category: workload.category.clone(),
            status: if workload_pass {
                "pass".to_string()
            } else {
                "fail".to_string()
            },
            score: workload_score,
            candidate: candidate_metrics,
            references,
            p50_ratio_to_best_ref: p50_ratio,
            p95_ratio_to_best_ref: p95_ratio,
            rss_ratio_to_best_ref: rss_ratio,
        });

        weighted_p50_sum += workload.weight * p50_score;
        weighted_p95_sum += workload.weight * p95_score;
        weighted_rss_sum += workload.weight * rss_score;
        workload_weight_sum += workload.weight;
    }

    let (p50_avg, p95_avg, rss_avg) = if workload_weight_sum <= f64::EPSILON {
        (0.0, 0.0, 0.0)
    } else {
        (
            weighted_p50_sum / workload_weight_sum,
            weighted_p95_sum / workload_weight_sum,
            weighted_rss_sum / workload_weight_sum,
        )
    };

    let candidate_artifact_size = fs::metadata(&args.bin_path).ok().map(|meta| meta.len());
    let best_ref_artifact_size =
        best_reference_metric_u64(baseline_bundle, &contract.reference_targets, |target| {
            target.artifact_size_bytes
        });
    let artifact_ratio = candidate_artifact_size
        .zip(best_ref_artifact_size)
        .and_then(|(candidate, reference)| ratio(candidate as f64, reference as f64));
    let artifact_score = score_ratio(artifact_ratio, budget_ratio);

    let candidate_compile_latency = args.compile_latency_ms;
    let best_ref_compile_latency =
        best_reference_metric_u64(baseline_bundle, &contract.reference_targets, |target| {
            target.compile_latency_ms
        });
    let compile_ratio = candidate_compile_latency
        .zip(best_ref_compile_latency)
        .and_then(|(candidate, reference)| ratio(candidate as f64, reference as f64));
    let compile_score = score_ratio(compile_ratio, budget_ratio);

    let overall_score = compute_overall_score(
        &contract.metric_weights,
        p50_avg,
        p95_avg,
        rss_avg,
        artifact_score,
        compile_score,
    );

    if let Some(value) = artifact_ratio {
        if value > budget_ratio {
            failure_reasons.push(format!(
                "artifact size ratio {:.3} exceeds relative budget {:.3}",
                value, budget_ratio
            ));
        }
    }

    if contract.metric_weights.compile_latency > 0.0 && candidate_compile_latency.is_none() {
        failure_reasons
            .push("missing --compile-latency-ms value for performance contract".to_string());
    } else if let Some(value) = compile_ratio {
        if value > budget_ratio {
            failure_reasons.push(format!(
                "compile latency ratio {:.3} exceeds relative budget {:.3}",
                value, budget_ratio
            ));
        }
    }

    let slo = evaluate_slo(
        &contract.slo,
        &manifest.workload,
        workload_reports,
        candidate_artifact_size,
        candidate_compile_latency,
    );

    if slo.status == "fail" {
        failure_reasons.extend(slo.failures.clone());
    }

    if overall_score < contract.pass_threshold {
        failure_reasons.push(format!(
            "overall score {:.3} is below pass threshold {:.3}",
            overall_score, contract.pass_threshold
        ));
    }

    failure_reasons.sort();
    failure_reasons.dedup();

    CompetitiveSummary {
        baseline_path: contract.baseline_path.clone(),
        candidate_target,
        reference_targets: contract.reference_targets.clone(),
        relative_budget_pct: contract.relative_budget_pct,
        pass_threshold: contract.pass_threshold,
        overall_score,
        status: if failure_reasons.is_empty() {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        workload_scores,
        slo,
        artifact_size_bytes: candidate_artifact_size,
        compile_latency_ms: candidate_compile_latency,
        artifact_size_score: artifact_score,
        compile_latency_score: compile_score,
        failure_reasons,
        baseline_metadata: baseline_bundle.metadata.clone(),
    }
}

fn compute_overall_score(
    weights: &MetricWeights,
    p50_avg: f64,
    p95_avg: f64,
    rss_avg: f64,
    artifact_score: Option<f64>,
    compile_score: Option<f64>,
) -> f64 {
    let mut weighted_sum =
        (p50_avg * weights.latency_p50) + (p95_avg * weights.latency_p95) + (rss_avg * weights.rss);
    let mut weight_sum = weights.latency_p50 + weights.latency_p95 + weights.rss;

    if let Some(score) = artifact_score {
        weighted_sum += score * weights.artifact_size;
        weight_sum += weights.artifact_size;
    }

    if let Some(score) = compile_score {
        weighted_sum += score * weights.compile_latency;
        weight_sum += weights.compile_latency;
    }

    if weight_sum <= f64::EPSILON {
        0.0
    } else {
        weighted_sum / weight_sum
    }
}

fn absolute_workload_failures(workload: &Workload, report: &WorkloadReport) -> Vec<String> {
    let mut failures = Vec::new();
    if report.p50_exceeded {
        failures.push(format!(
            "workload '{}' exceeded absolute p50 threshold {}ms",
            workload.name, workload.threshold_p50_ms
        ));
    }
    if report.p95_exceeded {
        failures.push(format!(
            "workload '{}' exceeded absolute p95 threshold {}ms",
            workload.name, workload.threshold_p95_ms
        ));
    }

    if report.rss_exceeded == Some(true) {
        if let Some(threshold) = workload.threshold_rss_kb {
            failures.push(format!(
                "workload '{}' exceeded absolute RSS threshold {}KB",
                workload.name, threshold
            ));
        }
    }

    if report.status == "error" {
        failures.push(format!("workload '{}' execution failed", workload.name));
    }

    failures
}

fn baseline_workload_metrics(
    bundle: &BaselineBundle,
    target_name: &str,
    workload_name: &str,
) -> Option<CompetitiveMetrics> {
    let target = bundle
        .targets
        .iter()
        .find(|target| target.name == target_name)?;
    let metrics = target
        .workloads
        .iter()
        .find(|workload| workload.name == workload_name)?;

    Some(CompetitiveMetrics {
        p50_ms: Some(metrics.p50_ms),
        p95_ms: Some(metrics.p95_ms),
        peak_rss_kb: metrics.peak_rss_kb,
    })
}

fn best_reference_metric_u64<F>(
    bundle: &BaselineBundle,
    reference_targets: &[String],
    selector: F,
) -> Option<u64>
where
    F: Fn(&BaselineTarget) -> Option<u64>,
{
    reference_targets
        .iter()
        .filter_map(|target_name| {
            bundle
                .targets
                .iter()
                .find(|target| &target.name == target_name)
        })
        .filter_map(selector)
        .min()
}
