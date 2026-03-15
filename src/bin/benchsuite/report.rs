use std::fs;
use std::path::Path;
use std::process::Command;

use crate::model::{HostMetadata, SuiteReport, Workload, WorkloadReport};

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
