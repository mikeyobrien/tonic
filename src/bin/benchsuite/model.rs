use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub const DEFAULT_TARGET_NAME: &str = "interpreter";

fn default_mode() -> String {
    "warm".to_string()
}

fn default_workload_target() -> String {
    DEFAULT_TARGET_NAME.to_string()
}

fn default_workload_weight() -> f64 {
    1.0
}

fn default_candidate_target() -> String {
    DEFAULT_TARGET_NAME.to_string()
}

fn default_reference_targets() -> Vec<String> {
    vec!["rust".to_string(), "go".to_string()]
}

fn default_relative_budget_pct() -> f64 {
    20.0
}

fn default_pass_threshold() -> f64 {
    0.8
}

fn default_metric_weights() -> MetricWeights {
    MetricWeights {
        latency_p50: 0.35,
        latency_p95: 0.25,
        rss: 0.2,
        artifact_size: 0.1,
        compile_latency: 0.1,
    }
}

#[derive(Debug, Deserialize)]
pub struct SuiteManifest {
    pub workload: Vec<Workload>,
    #[serde(default)]
    pub performance_contract: Option<PerformanceContract>,
}

#[derive(Debug, Deserialize)]
pub struct Workload {
    pub name: String,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default = "default_mode")]
    pub mode: String,
    #[serde(default = "default_workload_target")]
    pub target: String,
    #[serde(default)]
    pub source: Option<String>,
    pub threshold_p50_ms: u64,
    pub threshold_p95_ms: u64,
    #[serde(default)]
    pub threshold_rss_kb: Option<u64>,
    #[serde(default = "default_workload_weight")]
    pub weight: f64,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RunStats {
    pub p50_ms: f64,
    pub p95_ms: f64,
    pub samples_ms: Vec<f64>,
    pub peak_rss_kb: Option<u64>,
}

#[derive(Debug, Serialize, Clone)]
pub struct WorkloadReport {
    pub name: String,
    pub command: Vec<String>,
    pub mode: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    pub status: String,
    pub threshold_p50_ms: u64,
    pub threshold_p95_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold_rss_kb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f64>,
    pub p50_ms: Option<f64>,
    pub p95_ms: Option<f64>,
    pub p50_exceeded: bool,
    pub p95_exceeded: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_exceeded: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_threshold_p50_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_threshold_p95_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_rss_kb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples_ms: Option<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct SuiteReport {
    pub suite_path: String,
    pub bin_path: String,
    pub runs: usize,
    pub warmup_runs: usize,
    pub status: String,
    pub workloads: Vec<WorkloadReport>,
    pub metadata: HostMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub performance_contract: Option<CompetitiveSummary>,
}

#[derive(Debug, Clone)]
pub struct CliArgs {
    pub bin_path: PathBuf,
    pub manifest_path: PathBuf,
    pub runs: usize,
    pub warmup_runs: usize,
    pub enforce: bool,
    pub calibrate: bool,
    pub calibrate_margin_pct: u64,
    pub json_out: PathBuf,
    pub markdown_out: Option<PathBuf>,
    pub compile_latency_ms: Option<u64>,
    pub target_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct HostMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub captured_at_utc: Option<String>,
    pub os: String,
    pub arch: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kernel: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpu_model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rustc_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub go_version: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PerformanceContract {
    pub baseline_path: String,
    #[serde(default = "default_candidate_target")]
    pub candidate_target: String,
    #[serde(default = "default_reference_targets")]
    pub reference_targets: Vec<String>,
    #[serde(default = "default_relative_budget_pct")]
    pub relative_budget_pct: f64,
    #[serde(default = "default_pass_threshold")]
    pub pass_threshold: f64,
    #[serde(default = "default_metric_weights")]
    pub metric_weights: MetricWeights,
    #[serde(default)]
    pub slo: NativeSlo,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MetricWeights {
    pub latency_p50: f64,
    pub latency_p95: f64,
    pub rss: f64,
    pub artifact_size: f64,
    pub compile_latency: f64,
}

#[derive(Debug, Deserialize, Clone, Default, Serialize)]
pub struct NativeSlo {
    #[serde(default)]
    pub startup_p50_ms: Option<u64>,
    #[serde(default)]
    pub runtime_p50_ms: Option<u64>,
    #[serde(default)]
    pub runtime_p95_ms: Option<u64>,
    #[serde(default)]
    pub rss_kb: Option<u64>,
    #[serde(default)]
    pub artifact_size_bytes: Option<u64>,
    #[serde(default)]
    pub compile_latency_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct CompetitiveSummary {
    pub baseline_path: String,
    pub candidate_target: String,
    pub reference_targets: Vec<String>,
    pub relative_budget_pct: f64,
    pub pass_threshold: f64,
    pub overall_score: f64,
    pub status: String,
    pub workload_scores: Vec<CompetitiveWorkloadScore>,
    pub slo: SloEvaluation,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_latency_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_size_score: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_latency_score: Option<f64>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failure_reasons: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baseline_metadata: Option<HostMetadata>,
}

#[derive(Debug, Serialize)]
pub struct CompetitiveWorkloadScore {
    pub name: String,
    pub weight: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub status: String,
    pub score: f64,
    pub candidate: CompetitiveMetrics,
    pub references: Vec<NamedCompetitiveMetrics>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50_ratio_to_best_ref: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95_ratio_to_best_ref: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_ratio_to_best_ref: Option<f64>,
}

#[derive(Debug, Serialize)]
pub struct NamedCompetitiveMetrics {
    pub target: String,
    pub metrics: CompetitiveMetrics,
}

#[derive(Debug, Serialize)]
pub struct CompetitiveMetrics {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p50_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub p95_ms: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peak_rss_kb: Option<u64>,
}

#[derive(Debug, Serialize)]
pub struct SloEvaluation {
    pub status: String,
    pub thresholds: NativeSlo,
    pub measured: SloMeasured,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failures: Vec<String>,
}

#[derive(Debug, Serialize, Default)]
pub struct SloMeasured {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub startup_p50_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_p50_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_p95_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_kb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifact_size_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_latency_ms: Option<u64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BaselineBundle {
    #[serde(default)]
    pub metadata: Option<HostMetadata>,
    pub targets: Vec<BaselineTarget>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BaselineTarget {
    pub name: String,
    #[serde(default)]
    pub artifact_size_bytes: Option<u64>,
    #[serde(default)]
    pub compile_latency_ms: Option<u64>,
    pub workloads: Vec<BaselineWorkload>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BaselineWorkload {
    pub name: String,
    pub p50_ms: f64,
    pub p95_ms: f64,
    #[serde(default)]
    pub peak_rss_kb: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_manifest_supports_competitive_contract() {
        let fixture = r#"
        [performance_contract]
        baseline_path = "benchmarks/native-compiler-baselines.json"
        pass_threshold = 0.9
        relative_budget_pct = 25

          [performance_contract.metric_weights]
          latency_p50 = 0.3
          latency_p95 = 0.3
          rss = 0.2
          artifact_size = 0.1
          compile_latency = 0.1

          [performance_contract.slo]
          startup_p50_ms = 50
          runtime_p50_ms = 15
          runtime_p95_ms = 25
          rss_kb = 30720
          artifact_size_bytes = 16777216
          compile_latency_ms = 1500

        [[workload]]
        name = "run_control_flow"
        command = ["run", "examples/parity/06-control-flow/for_multi_generator.tn"]
        mode = "cold"
        threshold_p50_ms = 50
        threshold_p95_ms = 80
        threshold_rss_kb = 30720
        weight = 1.5
        category = "startup"
        "#;

        let suite: SuiteManifest = toml::from_str(fixture).expect("manifest should parse");
        let contract = suite
            .performance_contract
            .expect("contract should be present");
        assert_eq!(
            contract.baseline_path,
            "benchmarks/native-compiler-baselines.json"
        );
        assert_eq!(contract.pass_threshold, 0.9);
        assert_eq!(contract.relative_budget_pct, 25.0);
        assert_eq!(contract.slo.startup_p50_ms, Some(50));
        assert_eq!(suite.workload[0].threshold_rss_kb, Some(30720));
        assert_eq!(suite.workload[0].category.as_deref(), Some("startup"));
        assert_eq!(suite.workload[0].weight, 1.5);
    }

    #[test]
    fn parse_manifest_defaults_workload_weight() {
        let fixture = r#"
        [[workload]]
        name = "run_sample"
        command = ["run", "examples/sample.tn"]
        threshold_p50_ms = 100
        threshold_p95_ms = 250
        "#;

        let suite: SuiteManifest = toml::from_str(fixture).expect("manifest should parse");
        assert_eq!(suite.workload[0].weight, 1.0);
        assert_eq!(suite.workload[0].mode, "warm");
        assert_eq!(suite.workload[0].target, DEFAULT_TARGET_NAME);
        assert!(suite.workload[0].source.is_none());
    }

    #[test]
    fn parse_manifest_supports_compiled_target_workloads() {
        let fixture = r#"
        [[workload]]
        name = "run_native_budgeting"
        mode = "warm"
        target = "compiled"
        source = "examples/parity/02-operators/arithmetic_basic.tn"
        threshold_p50_ms = 10
        threshold_p95_ms = 20
        "#;

        let suite: SuiteManifest = toml::from_str(fixture).expect("manifest should parse");
        assert_eq!(suite.workload.len(), 1);
        assert_eq!(suite.workload[0].target, "compiled");
        assert_eq!(
            suite.workload[0].source.as_deref(),
            Some("examples/parity/02-operators/arithmetic_basic.tn")
        );
        assert!(suite.workload[0].command.is_empty());
    }
}
