use serde::Deserialize;
use std::collections::BTreeMap;
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

#[derive(Debug)]
struct RunStats {
    p50_ms: f64,
    p95_ms: f64,
    samples: Vec<f64>,
}

fn compute_percentile(mut samples: Vec<f64>, percentile: f64) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if percentile <= 0.0 {
        return samples[0];
    }
    if percentile >= 100.0 {
        return samples[samples.len() - 1];
    }
    let idx = (percentile / 100.0 * (samples.len() - 1) as f64).round() as usize;
    samples[idx]
}

fn run_workload(binary_path: &Path, workload: &Workload) -> Result<RunStats, String> {
    let runs = 10;
    let mut samples = Vec::new();

    // Warmup
    for _ in 0..2 {
        let _ = Command::new(binary_path)
            .args(&workload.command)
            .output()
            .map_err(|e| format!("Failed to execute {}: {}", binary_path.display(), e))?;
    }

    // Measured runs
    for _ in 0..runs {
        let start = Instant::now();
        let output = Command::new(binary_path)
            .args(&workload.command)
            .output()
            .map_err(|e| format!("Failed to execute {}: {}", binary_path.display(), e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Workload failed: {}", stderr));
        }
        let duration = start.elapsed().as_secs_f64() * 1000.0;
        samples.push(duration);
    }

    Ok(RunStats {
        p50_ms: compute_percentile(samples.clone(), 50.0),
        p95_ms: compute_percentile(samples.clone(), 95.0),
        samples,
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let mut binary_path = PathBuf::from("target/debug/tonic");
    let mut enforce = false;
    let mut manifest_path = PathBuf::from("benchmarks/suite.toml");

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--bin" => {
                i += 1;
                if i < args.len() {
                    binary_path = PathBuf::from(&args[i]);
                }
            }
            "--enforce" => {
                enforce = true;
            }
            "--manifest" => {
                i += 1;
                if i < args.len() {
                    manifest_path = PathBuf::from(&args[i]);
                }
            }
            _ => {}
        }
        i += 1;
    }

    if !manifest_path.exists() {
        eprintln!("Error: manifest not found at {}", manifest_path.display());
        std::process::exit(1);
    }

    let manifest_str = fs::read_to_string(&manifest_path).expect("Failed to read manifest");
    let suite: Suite = toml::from_str(&manifest_str).expect("Failed to parse manifest");

    let mut has_failures = false;
    let mut results = BTreeMap::new();

    for workload in &suite.workload {
        println!("Running workload: {}", workload.name);
        match run_workload(&binary_path, workload) {
            Ok(stats) => {
                println!(
                    "  p50: {:.2} ms (threshold: {} ms)",
                    stats.p50_ms, workload.threshold_p50_ms
                );
                println!(
                    "  p95: {:.2} ms (threshold: {} ms)",
                    stats.p95_ms, workload.threshold_p95_ms
                );

                let mut p50_exceeded = false;
                let mut p95_exceeded = false;

                if stats.p50_ms > workload.threshold_p50_ms as f64 {
                    println!("  ERROR: p50 threshold exceeded!");
                    p50_exceeded = true;
                    has_failures = true;
                }
                if stats.p95_ms > workload.threshold_p95_ms as f64 {
                    println!("  ERROR: p95 threshold exceeded!");
                    p95_exceeded = true;
                    has_failures = true;
                }

                let val = serde_json::json!({
                    "p50_ms": stats.p50_ms,
                    "p95_ms": stats.p95_ms,
                    "threshold_p50_ms": workload.threshold_p50_ms,
                    "threshold_p95_ms": workload.threshold_p95_ms,
                    "p50_exceeded": p50_exceeded,
                    "p95_exceeded": p95_exceeded,
                    "samples": stats.samples,
                });
                results.insert(workload.name.clone(), val);
            }
            Err(e) => {
                eprintln!("  ERROR: {}", e);
                has_failures = true;
            }
        }
    }

    let summary = serde_json::json!({
        "results": results,
    });
    fs::write(
        "benchmarks/summary.json",
        serde_json::to_string_pretty(&summary).unwrap(),
    )
    .unwrap();
    println!("Wrote summary to benchmarks/summary.json");

    if enforce && has_failures {
        eprintln!("One or more workloads failed or exceeded thresholds in enforce mode.");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_percentile() {
        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(compute_percentile(samples.clone(), 50.0), 30.0);
        assert_eq!(compute_percentile(samples.clone(), 95.0), 50.0);

        let samples = vec![10.0, 20.0, 30.0, 40.0, 50.0, 60.0, 70.0, 80.0, 90.0, 100.0];
        assert_eq!(compute_percentile(samples.clone(), 50.0), 60.0);
        assert_eq!(compute_percentile(samples.clone(), 95.0), 100.0);
    }

    #[test]
    fn test_manifest_parsing() {
        let toml_str = r#"
        [[workload]]
        name = "test"
        command = ["run", "foo.tn"]
        threshold_p50_ms = 100
        threshold_p95_ms = 200
        "#;
        let suite: Suite = toml::from_str(toml_str).unwrap();
        assert_eq!(suite.workload.len(), 1);
        assert_eq!(suite.workload[0].name, "test");
        assert_eq!(suite.workload[0].command, vec!["run", "foo.tn"]);
        assert_eq!(suite.workload[0].threshold_p50_ms, 100);
        assert_eq!(suite.workload[0].threshold_p95_ms, 200);
    }

    #[test]
    fn test_threshold_evaluation_logic() {
        let workload = Workload {
            name: "test".to_string(),
            command: vec![],
            threshold_p50_ms: 100,
            threshold_p95_ms: 200,
        };
        let stats = RunStats {
            p50_ms: 150.0,
            p95_ms: 190.0,
            samples: vec![],
        };
        assert!(stats.p50_ms > workload.threshold_p50_ms as f64);
        assert!(!(stats.p95_ms > workload.threshold_p95_ms as f64));
    }
}
