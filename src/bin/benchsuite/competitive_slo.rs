use crate::model::{NativeSlo, SloEvaluation, SloMeasured, Workload, WorkloadReport};
use crate::runtime::compute_percentile;
use std::collections::HashMap;

pub fn evaluate_slo(
    slo: &NativeSlo,
    workloads: &[Workload],
    reports: &[WorkloadReport],
    artifact_size_bytes: Option<u64>,
    compile_latency_ms: Option<u64>,
) -> SloEvaluation {
    let reports_by_name = reports
        .iter()
        .map(|report| (report.name.as_str(), report))
        .collect::<HashMap<_, _>>();

    let mut startup_samples = Vec::new();
    let mut runtime_p50_samples = Vec::new();
    let mut runtime_p95_samples = Vec::new();
    let mut rss_samples = Vec::new();

    for workload in workloads {
        let Some(report) = reports_by_name.get(workload.name.as_str()) else {
            continue;
        };

        let is_startup = workload
            .category
            .as_deref()
            .map(|value| value == "startup")
            .unwrap_or_else(|| workload.mode == "cold");

        if is_startup {
            if let Some(p50) = report.p50_ms {
                startup_samples.push(p50);
            }
        } else {
            if let Some(p50) = report.p50_ms {
                runtime_p50_samples.push(p50);
            }
            if let Some(p95) = report.p95_ms {
                runtime_p95_samples.push(p95);
            }
        }

        if let Some(rss) = report.peak_rss_kb {
            rss_samples.push(rss);
        }
    }

    let measured = SloMeasured {
        startup_p50_ms: max_as_u64(&startup_samples),
        runtime_p50_ms: percentile_as_u64(runtime_p50_samples, 50.0),
        runtime_p95_ms: max_as_u64(&runtime_p95_samples),
        rss_kb: rss_samples.iter().max().copied(),
        artifact_size_bytes,
        compile_latency_ms,
    };

    let mut failures = Vec::new();
    validate_slo_value(
        "startup_p50_ms",
        slo.startup_p50_ms,
        measured.startup_p50_ms,
        &mut failures,
    );
    validate_slo_value(
        "runtime_p50_ms",
        slo.runtime_p50_ms,
        measured.runtime_p50_ms,
        &mut failures,
    );
    validate_slo_value(
        "runtime_p95_ms",
        slo.runtime_p95_ms,
        measured.runtime_p95_ms,
        &mut failures,
    );
    validate_slo_value("rss_kb", slo.rss_kb, measured.rss_kb, &mut failures);
    validate_slo_value(
        "artifact_size_bytes",
        slo.artifact_size_bytes,
        measured.artifact_size_bytes,
        &mut failures,
    );
    validate_slo_value(
        "compile_latency_ms",
        slo.compile_latency_ms,
        measured.compile_latency_ms,
        &mut failures,
    );

    SloEvaluation {
        status: if failures.is_empty() {
            "pass".to_string()
        } else {
            "fail".to_string()
        },
        thresholds: slo.clone(),
        measured,
        failures,
    }
}

fn validate_slo_value(
    metric_name: &str,
    threshold: Option<u64>,
    measured: Option<u64>,
    failures: &mut Vec<String>,
) {
    let Some(threshold) = threshold else {
        return;
    };

    let Some(measured) = measured else {
        failures.push(format!(
            "missing measured value for SLO metric {metric_name}"
        ));
        return;
    };

    if measured > threshold {
        failures.push(format!(
            "SLO metric {metric_name} exceeded threshold: measured={measured}, threshold={threshold}"
        ));
    }
}

fn max_as_u64(values: &[f64]) -> Option<u64> {
    values
        .iter()
        .copied()
        .max_by(|a, b| a.total_cmp(b))
        .map(ceil_ms)
}

fn percentile_as_u64(values: Vec<f64>, percentile: f64) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    Some(ceil_ms(compute_percentile(values, percentile)))
}

fn ceil_ms(value: f64) -> u64 {
    value.ceil() as u64
}
