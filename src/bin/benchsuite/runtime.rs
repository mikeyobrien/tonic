use crate::model::{HostMetadata, RunStats, SuiteReport, Workload, WorkloadReport};
use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

pub fn compute_percentile(mut samples: Vec<f64>, percentile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    if percentile <= 0.0 {
        return samples[0];
    }
    if percentile >= 100.0 {
        return samples[samples.len() - 1];
    }

    let idx = (percentile / 100.0 * (samples.len() - 1) as f64).round() as usize;
    samples[idx]
}

/// Compute a robust p95 using Tukey upper-fence winsorization.
///
/// Before computing the 95th-percentile, this caps a *single isolated* sample
/// above the Tukey upper fence (Q3 + 1.5 × IQR) to the fence value.
///
/// If multiple samples exceed the fence, we leave the distribution unchanged
/// to avoid masking meaningful tail regressions. This targets one-off
/// scheduling spikes while preserving persistent tail signal.
///
/// p50 is intentionally computed via the plain `compute_percentile` path; it
/// is naturally robust to tail outliers and should not be altered.
pub fn compute_robust_p95(mut samples: Vec<f64>) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }

    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));

    let q1 = percentile_of_sorted(&samples, 25.0);
    let q3 = percentile_of_sorted(&samples, 75.0);
    let iqr = q3 - q1;
    let upper_fence = q3 + 1.5 * iqr;

    // Winsorize only if there is exactly one fence outlier.
    let outlier_indices: Vec<usize> = samples
        .iter()
        .enumerate()
        .filter_map(|(idx, value)| {
            if *value > upper_fence {
                Some(idx)
            } else {
                None
            }
        })
        .collect();

    if outlier_indices.len() == 1 {
        let idx = outlier_indices[0];
        samples[idx] = upper_fence;
    }

    compute_percentile(samples, 95.0)
}

/// Compute a percentile on a pre-sorted slice without re-sorting.
fn percentile_of_sorted(sorted: &[f64], percentile: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    if percentile <= 0.0 {
        return sorted[0];
    }
    if percentile >= 100.0 {
        return sorted[sorted.len() - 1];
    }
    let idx = (percentile / 100.0 * (sorted.len() - 1) as f64).round() as usize;
    sorted[idx]
}

pub fn requires_cache_clear(mode: &str) -> bool {
    mode == "cold"
}

pub fn calculate_calibrated_threshold(value: f64, margin_pct: u64) -> u64 {
    let margin_multiplier = 1.0 + (margin_pct as f64 / 100.0);
    (value * margin_multiplier).ceil() as u64 + 1
}

pub fn clear_cache() {
    let _ = fs::remove_dir_all(".tonic/cache");
}

#[derive(Debug, Clone)]
struct PreparedWorkload {
    executable: PathBuf,
    args: Vec<String>,
}

fn get_time_command() -> Option<String> {
    if Path::new("/usr/bin/time").exists() {
        return Some("/usr/bin/time".to_string());
    }

    if let Ok(paths) = std::env::var("PATH") {
        for path in std::env::split_paths(&paths) {
            let time_cmd = path.join("time");
            if time_cmd.is_file() {
                return Some(time_cmd.to_string_lossy().to_string());
            }
        }
    }

    None
}

fn measure_rss(prepared: &PreparedWorkload, workload: &Workload) -> Option<u64> {
    let time_cmd = get_time_command()?;

    if requires_cache_clear(&workload.mode) {
        clear_cache();
    }

    let output = Command::new(&time_cmd)
        .arg("-v")
        .arg(&prepared.executable)
        .args(&prepared.args)
        .output()
        .ok()?;

    let stderr = String::from_utf8_lossy(&output.stderr);
    for line in stderr.lines() {
        if line.contains("Maximum resident set size (kbytes):") {
            if let Some(val_str) = line.split(':').nth(1) {
                if let Ok(val) = val_str.trim().parse::<u64>() {
                    return Some(val);
                }
            }
        }
    }

    None
}

fn prepare_workload(binary_path: &Path, workload: &Workload) -> Result<PreparedWorkload, String> {
    match workload.target.as_str() {
        "interpreter" => {
            if workload.command.is_empty() {
                return Err(format!(
                    "workload '{}' target=interpreter requires a non-empty command",
                    workload.name
                ));
            }

            Ok(PreparedWorkload {
                executable: binary_path.to_path_buf(),
                args: workload.command.clone(),
            })
        }
        "compiled" => {
            let Some(source) = workload.source.as_ref() else {
                return Err(format!(
                    "workload '{}' target=compiled requires a source path",
                    workload.name
                ));
            };

            let executable = compiled_workload_output_path(workload);
            if let Some(parent) = executable.parent() {
                fs::create_dir_all(parent).map_err(|error| {
                    format!(
                        "failed to create compiled workload directory {}: {error}",
                        parent.display()
                    )
                })?;
            }

            let output = Command::new(binary_path)
                .args(["compile", source, "--out"])
                .arg(&executable)
                .output()
                .map_err(|error| {
                    format!(
                        "failed to prepare compiled workload '{}' via {}: {error}",
                        workload.name,
                        binary_path.display()
                    )
                })?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let stdout = String::from_utf8_lossy(&output.stdout);
                return Err(format!(
                    "workload '{}' failed during compile preparation: {}{}{}",
                    workload.name,
                    stderr.trim(),
                    if stderr.trim().is_empty() || stdout.trim().is_empty() {
                        ""
                    } else {
                        " | "
                    },
                    stdout.trim()
                ));
            }

            Ok(PreparedWorkload {
                executable,
                args: workload.command.clone(),
            })
        }
        other => Err(format!(
            "workload '{}' has unsupported target '{}' (expected interpreter or compiled)",
            workload.name, other
        )),
    }
}

fn compiled_workload_output_path(workload: &Workload) -> PathBuf {
    let mut path = PathBuf::from(".tonic/bench-compiled");
    path.push(sanitize_workload_name(&workload.name));

    if cfg!(windows) {
        path.set_extension("exe");
    }

    path
}

fn sanitize_workload_name(name: &str) -> String {
    let mut sanitized = String::with_capacity(name.len());

    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            sanitized.push(ch);
        } else {
            sanitized.push('_');
        }
    }

    if sanitized.is_empty() {
        "workload".to_string()
    } else {
        sanitized
    }
}

pub fn run_workload(
    binary_path: &Path,
    workload: &Workload,
    runs: usize,
    warmup_runs: usize,
) -> Result<RunStats, String> {
    let prepared = prepare_workload(binary_path, workload)?;

    for _ in 0..warmup_runs {
        if requires_cache_clear(&workload.mode) {
            clear_cache();
        }
        let output = Command::new(&prepared.executable)
            .args(&prepared.args)
            .output()
            .map_err(|error| {
                format!(
                    "failed to execute workload '{}' via {}: {error}",
                    workload.name,
                    prepared.executable.display()
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "workload '{}' failed during warmup: {}",
                workload.name,
                stderr.trim()
            ));
        }
    }

    let mut samples_ms = Vec::with_capacity(runs);
    for _ in 0..runs {
        if requires_cache_clear(&workload.mode) {
            clear_cache();
        }
        let start = Instant::now();
        let output = Command::new(&prepared.executable)
            .args(&prepared.args)
            .output()
            .map_err(|error| {
                format!(
                    "failed to execute workload '{}' via {}: {error}",
                    workload.name,
                    prepared.executable.display()
                )
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!(
                "workload '{}' failed during measured run: {}",
                workload.name,
                stderr.trim()
            ));
        }

        let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
        samples_ms.push(elapsed_ms);
    }

    let peak_rss_kb = measure_rss(&prepared, workload);

    Ok(RunStats {
        p50_ms: compute_percentile(samples_ms.clone(), 50.0),
        // Use Tukey-winsorized p95 so isolated scheduling spikes do not cause
        // spurious gate failures; see `compute_robust_p95` for the rationale.
        p95_ms: compute_robust_p95(samples_ms.clone()),
        samples_ms,
        peak_rss_kb,
    })
}

pub fn evaluate_thresholds(stats: &RunStats, workload: &Workload) -> (bool, bool, Option<bool>) {
    let p50_exceeded = stats.p50_ms > workload.threshold_p50_ms as f64;
    let p95_exceeded = stats.p95_ms > workload.threshold_p95_ms as f64;
    let rss_exceeded = workload
        .threshold_rss_kb
        .zip(stats.peak_rss_kb)
        .map(|(threshold, measured)| measured > threshold);

    (p50_exceeded, p95_exceeded, rss_exceeded)
}

pub fn write_json_report(path: &Path, report: &SuiteReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to serialize summary json: {error}"))?;

    fs::write(path, payload).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub fn write_markdown_report(path: &Path, report: &SuiteReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let mut markdown = String::new();
    markdown.push_str("# Tonic Benchmark Summary\n\n");
    markdown.push_str(&format!(
        "- suite: `{}`\n- binary: `{}`\n- runs: {}\n- warmup runs: {}\n- status: **{}**\n\n",
        report.suite_path, report.bin_path, report.runs, report.warmup_runs, report.status
    ));

    markdown.push_str("## Host Metadata\n\n");
    markdown.push_str(&format!(
        "- os/arch: `{}/{}`\n",
        report.metadata.os, report.metadata.arch
    ));
    if let Some(kernel) = &report.metadata.kernel {
        markdown.push_str(&format!("- kernel: `{kernel}`\n"));
    }
    if let Some(cpu_model) = &report.metadata.cpu_model {
        markdown.push_str(&format!("- cpu: `{cpu_model}`\n"));
    }
    if let Some(rustc) = &report.metadata.rustc_version {
        markdown.push_str(&format!("- rustc: `{rustc}`\n"));
    }
    if let Some(go) = &report.metadata.go_version {
        markdown.push_str(&format!("- go: `{go}`\n"));
    }
    markdown.push('\n');

    markdown.push_str("| Workload | Target | Source | Mode | Status | p50 (ms) | p95 (ms) | p50 threshold | p95 threshold | RSS (KB) | RSS threshold (KB) |\n");
    markdown.push_str("|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|\n");

    for workload in &report.workloads {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            workload.name,
            workload.target,
            workload.source.as_deref().unwrap_or("-"),
            workload.mode,
            workload.status,
            workload
                .p50_ms
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "-".to_string()),
            workload
                .p95_ms
                .map(|value| format!("{value:.2}"))
                .unwrap_or_else(|| "-".to_string()),
            workload.threshold_p50_ms,
            workload.threshold_p95_ms,
            workload
                .peak_rss_kb
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
            workload
                .threshold_rss_kb
                .map(|v| v.to_string())
                .unwrap_or_else(|| "-".to_string()),
        ));
    }

    if let Some(contract) = &report.performance_contract {
        markdown.push_str("\n## Native Compiler Contract\n\n");
        markdown.push_str(&format!(
            "- status: **{}**\n- overall score: `{:.3}` (threshold `{:.3}`)\n- relative budget: `{:.1}%`\n- candidate: `{}`\n- references: `{}`\n",
            contract.status,
            contract.overall_score,
            contract.pass_threshold,
            contract.relative_budget_pct,
            contract.candidate_target,
            contract.reference_targets.join(", "),
        ));

        if !contract.failure_reasons.is_empty() {
            markdown.push_str("\n### Failure Reasons\n\n");
            for reason in &contract.failure_reasons {
                markdown.push_str(&format!("- {reason}\n"));
            }
        }
    }

    fs::write(path, markdown)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

pub fn collect_host_metadata() -> HostMetadata {
    HostMetadata {
        captured_at_utc: Some(crate::utils::utc_timestamp()),
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        kernel: capture_command_output("uname", &["-r"]),
        cpu_model: detect_cpu_model(),
        rustc_version: capture_command_output("rustc", &["--version"]),
        go_version: capture_command_output("go", &["version"]),
    }
}

fn detect_cpu_model() -> Option<String> {
    let cpuinfo = fs::read_to_string("/proc/cpuinfo").ok()?;
    cpuinfo.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.trim() == "model name" {
            Some(value.trim().to_string())
        } else {
            None
        }
    })
}

fn capture_command_output(command: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(command).args(args).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn workload_report_from_error(workload: &Workload, error: String) -> WorkloadReport {
    WorkloadReport {
        name: workload.name.clone(),
        command: workload.command.clone(),
        mode: workload.mode.clone(),
        target: workload.target.clone(),
        source: workload.source.clone(),
        status: "error".to_string(),
        threshold_p50_ms: workload.threshold_p50_ms,
        threshold_p95_ms: workload.threshold_p95_ms,
        threshold_rss_kb: workload.threshold_rss_kb,
        category: workload.category.clone(),
        weight: Some(workload.weight),
        p50_ms: None,
        p95_ms: None,
        p50_exceeded: false,
        p95_exceeded: false,
        rss_exceeded: None,
        suggested_threshold_p50_ms: None,
        suggested_threshold_p95_ms: None,
        peak_rss_kb: None,
        error: Some(error),
        samples_ms: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_percentile_handles_bounds_and_midpoints() {
        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(compute_percentile(samples.clone(), 0.0), 10.0);
        assert_eq!(compute_percentile(samples.clone(), 50.0), 30.0);
        assert_eq!(compute_percentile(samples.clone(), 95.0), 50.0);
        assert_eq!(compute_percentile(samples, 100.0), 50.0);
    }

    #[test]
    fn compute_robust_p95_caps_isolated_outlier() {
        // 9 samples at 1.0ms + 1 scheduling spike at 50ms (10 total).
        // With 10 samples, p95 index = round(0.95 × 9) = 9, which is the spike.
        // The Tukey fence for IQR=0 collapses to Q3=1.0, capping the spike.
        let mut samples: Vec<f64> = vec![1.0; 9];
        samples.push(50.0);
        let robust = compute_robust_p95(samples.clone());
        let raw = compute_percentile(samples, 95.0);
        // Raw p95 lands on the spike; robust p95 should be lower.
        assert!(
            robust < raw,
            "robust p95 {robust} should be lower than raw p95 {raw}"
        );
        // The result must still represent real latency (not suppressed to zero).
        assert!(
            robust >= 1.0,
            "robust p95 {robust} should be at least 1.0ms"
        );
    }

    #[test]
    fn compute_robust_p95_preserves_sustained_regression_signal() {
        // All 15 samples are uniformly high (sustained regression, not outlier).
        let samples: Vec<f64> = vec![10.0; 15];
        let robust = compute_robust_p95(samples);
        // IQR is 0, fence = Q3 = 10.0, so nothing is winsorized; p95 stays at 10.0.
        assert_eq!(
            robust, 10.0,
            "sustained regression should not be winsorized away"
        );
    }

    #[test]
    fn compute_robust_p95_preserves_tail_signal_when_not_isolated() {
        // 20% of samples are slower. This is not an isolated outlier and should
        // remain visible in p95.
        let mut samples: Vec<f64> = vec![1.0; 12];
        samples.extend([2.0, 2.0, 2.0]);

        let robust = compute_robust_p95(samples.clone());
        let raw = compute_percentile(samples, 95.0);

        assert_eq!(raw, 2.0, "sanity: raw p95 should capture the slower tail");
        assert_eq!(
            robust, raw,
            "multiple high-tail samples must not be winsorized away"
        );
    }

    #[test]
    fn compute_robust_p95_equals_raw_when_no_outliers() {
        // Clean, tightly clustered samples — no winsorization should occur.
        let samples: Vec<f64> = (1..=15).map(|v| v as f64 * 0.5).collect();
        let robust = compute_robust_p95(samples.clone());
        let raw = compute_percentile(samples, 95.0);
        assert_eq!(robust, raw, "no outliers means robust == raw p95");
    }

    #[test]
    fn compute_robust_p95_handles_single_sample() {
        assert_eq!(compute_robust_p95(vec![42.0]), 42.0);
    }

    #[test]
    fn compute_robust_p95_handles_empty() {
        assert_eq!(compute_robust_p95(vec![]), 0.0);
    }

    #[test]
    fn threshold_evaluation_reports_exceeded_dimensions() {
        let workload = Workload {
            name: "w".to_string(),
            command: vec!["run".to_string(), "examples/sample.tn".to_string()],
            mode: "warm".to_string(),
            target: "interpreter".to_string(),
            source: None,
            threshold_p50_ms: 100,
            threshold_p95_ms: 200,
            threshold_rss_kb: Some(12_000),
            weight: 1.0,
            category: None,
        };

        let stats = RunStats {
            p50_ms: 120.0,
            p95_ms: 190.0,
            samples_ms: vec![120.0],
            peak_rss_kb: Some(12_500),
        };

        let (p50_exceeded, p95_exceeded, rss_exceeded) = evaluate_thresholds(&stats, &workload);
        assert!(p50_exceeded);
        assert!(!p95_exceeded);
        assert_eq!(rss_exceeded, Some(true));
    }

    #[test]
    fn test_requires_cache_clear() {
        assert!(requires_cache_clear("cold"));
        assert!(!requires_cache_clear("warm"));
        assert!(!requires_cache_clear("hot"));
        assert!(!requires_cache_clear(""));
    }

    #[test]
    fn test_calculate_calibrated_threshold() {
        assert_eq!(calculate_calibrated_threshold(100.0, 20), 121);
        assert_eq!(calculate_calibrated_threshold(50.5, 10), 57);
        assert_eq!(calculate_calibrated_threshold(10.0, 0), 11);
        assert_eq!(calculate_calibrated_threshold(200.0, 150), 501);
    }

    #[test]
    fn compiled_output_path_is_deterministic_and_sanitized() {
        let workload = Workload {
            name: "run compiled: budgeting".to_string(),
            command: Vec::new(),
            mode: "warm".to_string(),
            target: "compiled".to_string(),
            source: Some("examples/ergonomics/budgeting.tn".to_string()),
            threshold_p50_ms: 10,
            threshold_p95_ms: 20,
            threshold_rss_kb: None,
            weight: 1.0,
            category: None,
        };

        let path = compiled_workload_output_path(&workload);
        assert!(
            path.starts_with(std::path::Path::new(".tonic/bench-compiled")),
            "compiled bench output should be under .tonic/bench-compiled"
        );
        assert_eq!(
            path.file_name().and_then(|name| name.to_str()),
            Some("run_compiled__budgeting")
        );
    }
}
