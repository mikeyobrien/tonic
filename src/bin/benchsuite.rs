use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct Suite {
    workload: Vec<Workload>,
}

#[derive(Debug, Deserialize)]
struct Workload {
    name: String,
    command: Vec<String>,
    threshold_p50_ms: u64,
    threshold_p95_ms: u64,
}

#[derive(Debug, Clone)]
struct RunStats {
    p50_ms: f64,
    p95_ms: f64,
    samples_ms: Vec<f64>,
}

#[derive(Debug, Serialize)]
struct WorkloadReport {
    name: String,
    command: Vec<String>,
    status: String,
    threshold_p50_ms: u64,
    threshold_p95_ms: u64,
    p50_ms: Option<f64>,
    p95_ms: Option<f64>,
    p50_exceeded: bool,
    p95_exceeded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    samples_ms: Option<Vec<f64>>,
}

#[derive(Debug, Serialize)]
struct SuiteReport {
    suite_path: String,
    bin_path: String,
    runs: usize,
    warmup_runs: usize,
    status: String,
    workloads: Vec<WorkloadReport>,
}

#[derive(Debug, Clone)]
struct CliArgs {
    bin_path: PathBuf,
    manifest_path: PathBuf,
    runs: usize,
    warmup_runs: usize,
    enforce: bool,
    json_out: PathBuf,
    markdown_out: Option<PathBuf>,
}

fn parse_cli_args<I>(args: I) -> Result<CliArgs, String>
where
    I: IntoIterator<Item = String>,
{
    let mut bin_path = PathBuf::from("target/release/tonic");
    let mut manifest_path = PathBuf::from("benchmarks/suite.toml");
    let mut runs: usize = 15;
    let mut warmup_runs: usize = 3;
    let mut enforce = false;
    let mut json_out = PathBuf::from("benchmarks/summary.json");
    let mut markdown_out: Option<PathBuf> = None;

    let mut iter = args.into_iter();
    while let Some(flag) = iter.next() {
        match flag.as_str() {
            "--bin" => {
                let Some(value) = iter.next() else {
                    return Err("--bin requires a value".to_string());
                };
                bin_path = PathBuf::from(value);
            }
            "--manifest" => {
                let Some(value) = iter.next() else {
                    return Err("--manifest requires a value".to_string());
                };
                manifest_path = PathBuf::from(value);
            }
            "--runs" => {
                let Some(value) = iter.next() else {
                    return Err("--runs requires a value".to_string());
                };
                runs = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --runs value '{value}' (expected integer)"))?;
                if runs == 0 {
                    return Err("--runs must be >= 1".to_string());
                }
            }
            "--warmup" => {
                let Some(value) = iter.next() else {
                    return Err("--warmup requires a value".to_string());
                };
                warmup_runs = value
                    .parse::<usize>()
                    .map_err(|_| format!("invalid --warmup value '{value}' (expected integer)"))?;
            }
            "--json-out" => {
                let Some(value) = iter.next() else {
                    return Err("--json-out requires a value".to_string());
                };
                json_out = PathBuf::from(value);
            }
            "--markdown-out" => {
                let Some(value) = iter.next() else {
                    return Err("--markdown-out requires a value".to_string());
                };
                markdown_out = Some(PathBuf::from(value));
            }
            "--enforce" => enforce = true,
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            other => return Err(format!("unexpected argument '{other}'")),
        }
    }

    Ok(CliArgs {
        bin_path,
        manifest_path,
        runs,
        warmup_runs,
        enforce,
        json_out,
        markdown_out,
    })
}

fn print_help() {
    println!(
        "Usage:\n  benchsuite [--bin <path>] [--manifest <path>] [--runs <n>] [--warmup <n>] [--json-out <path>] [--markdown-out <path>] [--enforce]\n\
\nDefaults:\n  --bin target/release/tonic\n  --manifest benchmarks/suite.toml\n  --runs 15\n  --warmup 3\n  --json-out benchmarks/summary.json\n"
    );
}

fn compute_percentile(mut samples: Vec<f64>, percentile: f64) -> f64 {
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

fn run_workload(
    binary_path: &Path,
    workload: &Workload,
    runs: usize,
    warmup_runs: usize,
) -> Result<RunStats, String> {
    for _ in 0..warmup_runs {
        let output = Command::new(binary_path)
            .args(&workload.command)
            .output()
            .map_err(|error| {
                format!(
                    "failed to execute workload '{}' via {}: {error}",
                    workload.name,
                    binary_path.display()
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
        let start = Instant::now();
        let output = Command::new(binary_path)
            .args(&workload.command)
            .output()
            .map_err(|error| {
                format!(
                    "failed to execute workload '{}' via {}: {error}",
                    workload.name,
                    binary_path.display()
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

    Ok(RunStats {
        p50_ms: compute_percentile(samples_ms.clone(), 50.0),
        p95_ms: compute_percentile(samples_ms.clone(), 95.0),
        samples_ms,
    })
}

fn evaluate_thresholds(stats: &RunStats, workload: &Workload) -> (bool, bool) {
    (
        stats.p50_ms > workload.threshold_p50_ms as f64,
        stats.p95_ms > workload.threshold_p95_ms as f64,
    )
}

fn write_json_report(path: &Path, report: &SuiteReport) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(report)
        .map_err(|error| format!("failed to serialize summary json: {error}"))?;

    fs::write(path, payload).map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn write_markdown_report(path: &Path, report: &SuiteReport) -> Result<(), String> {
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

    markdown
        .push_str("| Workload | Status | p50 (ms) | p95 (ms) | p50 threshold | p95 threshold |\n");
    markdown.push_str("|---|---:|---:|---:|---:|---:|\n");

    for workload in &report.workloads {
        markdown.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            workload.name,
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
        ));
    }

    fs::write(path, markdown)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))
}

fn main() {
    let args = match parse_cli_args(env::args().skip(1)) {
        Ok(args) => args,
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

    let suite: Suite = match toml::from_str(&manifest_str) {
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

    let mut reports = Vec::new();
    let mut has_failures = false;

    for workload in &suite.workload {
        println!("running workload: {}", workload.name);

        let report = match run_workload(&args.bin_path, workload, args.runs, args.warmup_runs) {
            Ok(stats) => {
                let (p50_exceeded, p95_exceeded) = evaluate_thresholds(&stats, workload);
                if p50_exceeded || p95_exceeded {
                    has_failures = true;
                }

                WorkloadReport {
                    name: workload.name.clone(),
                    command: workload.command.clone(),
                    status: if p50_exceeded || p95_exceeded {
                        "threshold_exceeded".to_string()
                    } else {
                        "pass".to_string()
                    },
                    threshold_p50_ms: workload.threshold_p50_ms,
                    threshold_p95_ms: workload.threshold_p95_ms,
                    p50_ms: Some(stats.p50_ms),
                    p95_ms: Some(stats.p95_ms),
                    p50_exceeded,
                    p95_exceeded,
                    error: None,
                    samples_ms: Some(stats.samples_ms),
                }
            }
            Err(error) => {
                has_failures = true;
                WorkloadReport {
                    name: workload.name.clone(),
                    command: workload.command.clone(),
                    status: "error".to_string(),
                    threshold_p50_ms: workload.threshold_p50_ms,
                    threshold_p95_ms: workload.threshold_p95_ms,
                    p50_ms: None,
                    p95_ms: None,
                    p50_exceeded: false,
                    p95_exceeded: false,
                    error: Some(error),
                    samples_ms: None,
                }
            }
        };

        if let Some(p50) = report.p50_ms {
            println!("  p50={p50:.2}ms (threshold {}ms)", report.threshold_p50_ms);
        }
        if let Some(p95) = report.p95_ms {
            println!("  p95={p95:.2}ms (threshold {}ms)", report.threshold_p95_ms);
        }
        if let Some(error) = &report.error {
            println!("  error={error}");
        }

        reports.push(report);
    }

    let report = SuiteReport {
        suite_path: args.manifest_path.display().to_string(),
        bin_path: args.bin_path.display().to_string(),
        runs: args.runs,
        warmup_runs: args.warmup_runs,
        status: if has_failures {
            "fail".to_string()
        } else {
            "pass".to_string()
        },
        workloads: reports,
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
    fn parse_manifest_supports_expected_workload_shape() {
        let fixture = r#"
        [[workload]]
        name = "run_sample"
        command = ["run", "examples/sample.tn"]
        threshold_p50_ms = 100
        threshold_p95_ms = 250
        "#;

        let suite: Suite = toml::from_str(fixture).expect("manifest should parse");
        assert_eq!(suite.workload.len(), 1);
        assert_eq!(suite.workload[0].name, "run_sample");
        assert_eq!(suite.workload[0].command, vec!["run", "examples/sample.tn"]);
        assert_eq!(suite.workload[0].threshold_p50_ms, 100);
        assert_eq!(suite.workload[0].threshold_p95_ms, 250);
    }

    #[test]
    fn threshold_evaluation_reports_exceeded_dimensions() {
        let workload = Workload {
            name: "w".to_string(),
            command: vec!["run".to_string(), "examples/sample.tn".to_string()],
            threshold_p50_ms: 100,
            threshold_p95_ms: 200,
        };

        let stats = RunStats {
            p50_ms: 120.0,
            p95_ms: 190.0,
            samples_ms: vec![120.0],
        };

        let (p50_exceeded, p95_exceeded) = evaluate_thresholds(&stats, &workload);
        assert!(p50_exceeded);
        assert!(!p95_exceeded);
    }

    #[test]
    fn parse_cli_args_accepts_all_supported_flags() {
        let args = vec![
            "--bin".to_string(),
            "target/release/tonic".to_string(),
            "--manifest".to_string(),
            "benchmarks/suite.toml".to_string(),
            "--runs".to_string(),
            "9".to_string(),
            "--warmup".to_string(),
            "2".to_string(),
            "--json-out".to_string(),
            "benchmarks/out.json".to_string(),
            "--markdown-out".to_string(),
            "benchmarks/out.md".to_string(),
            "--enforce".to_string(),
        ];

        let parsed = parse_cli_args(args).expect("args should parse");
        assert_eq!(parsed.bin_path, PathBuf::from("target/release/tonic"));
        assert_eq!(parsed.manifest_path, PathBuf::from("benchmarks/suite.toml"));
        assert_eq!(parsed.runs, 9);
        assert_eq!(parsed.warmup_runs, 2);
        assert!(parsed.enforce);
        assert_eq!(parsed.json_out, PathBuf::from("benchmarks/out.json"));
        assert_eq!(
            parsed.markdown_out,
            Some(PathBuf::from("benchmarks/out.md"))
        );
    }
}
