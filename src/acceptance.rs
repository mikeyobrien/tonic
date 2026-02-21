use std::io::ErrorKind;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct AcceptanceMetadata {
    pub path: PathBuf,
    pub feature_files: Vec<PathBuf>,
    pub benchmark_metrics: Option<BenchmarkMetrics>,
    pub manual_evidence: ManualEvidenceRequirements,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchmarkMetrics {
    pub cold_start_p50_ms: u64,
    pub warm_start_p50_ms: u64,
    pub idle_rss_mb: u64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ManualEvidenceRequirements {
    pub auto: Vec<PathBuf>,
    pub mixed: Vec<PathBuf>,
    pub manual: Vec<PathBuf>,
}

impl ManualEvidenceRequirements {
    pub fn for_mode(&self, mode: &str) -> &[PathBuf] {
        match mode {
            "auto" => &self.auto,
            "mixed" => &self.mixed,
            "manual" => &self.manual,
            _ => &[],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureScenario {
    pub id: String,
    pub tags: Vec<String>,
}

pub fn load_acceptance_yaml(slice_id: &str) -> Result<AcceptanceMetadata, String> {
    let acceptance_path = acceptance_file_path(slice_id);

    let contents =
        std::fs::read_to_string(&acceptance_path).map_err(|error| match error.kind() {
            ErrorKind::NotFound => {
                format!("missing acceptance file {}", acceptance_path.display())
            }
            _ => format!(
                "failed to read acceptance file {}: {error}",
                acceptance_path.display()
            ),
        })?;

    let yaml = serde_yaml::from_str::<serde_yaml::Value>(&contents).map_err(|error| {
        format!(
            "invalid acceptance yaml {}: {error}",
            acceptance_path.display()
        )
    })?;

    let feature_files = parse_feature_files(&yaml, &acceptance_path)?;
    let benchmark_metrics = parse_benchmark_metrics(&yaml, &acceptance_path)?;
    let manual_evidence = parse_manual_evidence(&yaml, &acceptance_path)?;

    Ok(AcceptanceMetadata {
        path: acceptance_path,
        feature_files,
        benchmark_metrics,
        manual_evidence,
    })
}

pub fn load_feature_scenarios(feature_files: &[PathBuf]) -> Result<Vec<FeatureScenario>, String> {
    let mut scenarios = Vec::new();

    for feature_file in feature_files {
        let contents =
            std::fs::read_to_string(feature_file).map_err(|error| match error.kind() {
                ErrorKind::NotFound => {
                    format!("missing feature file {}", feature_file.display())
                }
                _ => format!(
                    "failed to read feature file {}: {error}",
                    feature_file.display()
                ),
            })?;

        scenarios.extend(parse_feature_scenarios(&contents));
    }

    Ok(scenarios)
}

pub fn parse_feature_scenarios(feature_file_contents: &str) -> Vec<FeatureScenario> {
    let mut scenarios = Vec::new();
    let mut pending_tags: Vec<String> = Vec::new();

    for line in feature_file_contents.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        if trimmed.starts_with('@') {
            pending_tags.extend(
                trimmed
                    .split_whitespace()
                    .filter(|value| value.starts_with('@'))
                    .map(ToString::to_string),
            );
            continue;
        }

        let scenario_name = trimmed
            .strip_prefix("Scenario:")
            .or_else(|| trimmed.strip_prefix("Scenario Outline:"));

        if let Some(name) = scenario_name {
            scenarios.push(FeatureScenario {
                id: name.trim().to_string(),
                tags: std::mem::take(&mut pending_tags),
            });
        }
    }

    scenarios
}

pub fn acceptance_file_path(slice_id: &str) -> PathBuf {
    Path::new("acceptance").join(format!("{slice_id}.yaml"))
}

fn parse_feature_files(
    yaml: &serde_yaml::Value,
    acceptance_path: &Path,
) -> Result<Vec<PathBuf>, String> {
    let Some(raw_feature_files) = yaml.get("feature_files") else {
        return Ok(Vec::new());
    };

    let feature_files = raw_feature_files.as_sequence().ok_or_else(|| {
        format!(
            "invalid acceptance yaml {}: field 'feature_files' must be a sequence",
            acceptance_path.display()
        )
    })?;

    let mut resolved = Vec::with_capacity(feature_files.len());

    for entry in feature_files {
        let raw_path = entry.as_str().ok_or_else(|| {
            format!(
                "invalid acceptance yaml {}: feature file entries must be strings",
                acceptance_path.display()
            )
        })?;

        resolved.push(resolve_acceptance_path(raw_path, acceptance_path));
    }

    Ok(resolved)
}

fn parse_benchmark_metrics(
    yaml: &serde_yaml::Value,
    acceptance_path: &Path,
) -> Result<Option<BenchmarkMetrics>, String> {
    let Some(raw_benchmark_metrics) = yaml.get("benchmark_metrics") else {
        return Ok(None);
    };

    if !raw_benchmark_metrics.is_mapping() {
        return Err(format!(
            "invalid acceptance yaml {}: field 'benchmark_metrics' must be a mapping",
            acceptance_path.display()
        ));
    }

    Ok(Some(BenchmarkMetrics {
        cold_start_p50_ms: parse_benchmark_metric_u64(
            raw_benchmark_metrics,
            "cold_start_p50_ms",
            acceptance_path,
        )?,
        warm_start_p50_ms: parse_benchmark_metric_u64(
            raw_benchmark_metrics,
            "warm_start_p50_ms",
            acceptance_path,
        )?,
        idle_rss_mb: parse_benchmark_metric_u64(
            raw_benchmark_metrics,
            "idle_rss_mb",
            acceptance_path,
        )?,
    }))
}

fn parse_benchmark_metric_u64(
    raw_benchmark_metrics: &serde_yaml::Value,
    key: &str,
    acceptance_path: &Path,
) -> Result<u64, String> {
    let Some(raw_value) = raw_benchmark_metrics.get(key) else {
        return Err(format!(
            "invalid acceptance yaml {}: field 'benchmark_metrics.{key}' is required",
            acceptance_path.display()
        ));
    };

    raw_value.as_u64().ok_or_else(|| {
        format!(
            "invalid acceptance yaml {}: field 'benchmark_metrics.{key}' must be an integer",
            acceptance_path.display()
        )
    })
}

fn parse_manual_evidence(
    yaml: &serde_yaml::Value,
    acceptance_path: &Path,
) -> Result<ManualEvidenceRequirements, String> {
    let Some(raw_manual_evidence) = yaml.get("manual_evidence") else {
        return Ok(ManualEvidenceRequirements::default());
    };

    if !raw_manual_evidence.is_mapping() {
        return Err(format!(
            "invalid acceptance yaml {}: field 'manual_evidence' must be a mapping",
            acceptance_path.display()
        ));
    }

    Ok(ManualEvidenceRequirements {
        auto: parse_manual_evidence_entries(raw_manual_evidence, "auto", acceptance_path)?,
        mixed: parse_manual_evidence_entries(raw_manual_evidence, "mixed", acceptance_path)?,
        manual: parse_manual_evidence_entries(raw_manual_evidence, "manual", acceptance_path)?,
    })
}

fn parse_manual_evidence_entries(
    raw_manual_evidence: &serde_yaml::Value,
    mode_key: &str,
    acceptance_path: &Path,
) -> Result<Vec<PathBuf>, String> {
    let Some(raw_entries) = raw_manual_evidence.get(mode_key) else {
        return Ok(Vec::new());
    };

    let entries = raw_entries.as_sequence().ok_or_else(|| {
        format!(
            "invalid acceptance yaml {}: field 'manual_evidence.{mode_key}' must be a sequence",
            acceptance_path.display()
        )
    })?;

    let mut resolved = Vec::with_capacity(entries.len());

    for entry in entries {
        let raw_path = entry.as_str().ok_or_else(|| {
            format!(
                "invalid acceptance yaml {}: field 'manual_evidence.{mode_key}' entries must be strings",
                acceptance_path.display()
            )
        })?;

        resolved.push(resolve_acceptance_path(raw_path, acceptance_path));
    }

    Ok(resolved)
}

fn resolve_acceptance_path(raw_path: &str, acceptance_path: &Path) -> PathBuf {
    let candidate = PathBuf::from(raw_path);

    if candidate.is_absolute() || candidate.starts_with("acceptance") {
        candidate
    } else {
        acceptance_path
            .parent()
            .unwrap_or(Path::new("."))
            .join(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        acceptance_file_path, load_acceptance_yaml, parse_feature_scenarios, AcceptanceMetadata,
    };
    use std::fs;
    use std::path::PathBuf;

    #[test]
    fn acceptance_file_path_is_stable() {
        assert_eq!(
            acceptance_file_path("step-01"),
            PathBuf::from("acceptance/step-01.yaml")
        );
    }

    #[test]
    fn parse_feature_scenarios_collects_ids_and_tags() {
        let scenarios = parse_feature_scenarios(
            "Feature: metadata\n\n  @auto @agent-manual\n  Scenario: smoke\n    Given parser output\n\n  @human-manual\n  Scenario Outline: detailed\n    Given richer output\n",
        );

        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios[0].id, "smoke");
        assert_eq!(scenarios[0].tags, ["@auto", "@agent-manual"]);
        assert_eq!(scenarios[1].id, "detailed");
        assert_eq!(scenarios[1].tags, ["@human-manual"]);
    }

    #[test]
    fn load_acceptance_yaml_resolves_relative_feature_paths() {
        let fixture_root = unique_fixture_root("acceptance-utils");
        fs::create_dir_all(fixture_root.join("acceptance/features"))
            .expect("fixture setup should create acceptance/features directory");

        fs::write(
            fixture_root.join("acceptance/step-01.yaml"),
            "slice_id: step-01\nfeature_files:\n  - features/step-01.feature\n",
        )
        .expect("fixture setup should write acceptance yaml");

        let previous_dir = std::env::current_dir().expect("cwd should be readable");
        std::env::set_current_dir(&fixture_root).expect("cwd should switch to fixture root");

        let metadata = load_acceptance_yaml("step-01");

        std::env::set_current_dir(previous_dir).expect("cwd should restore to original location");

        let AcceptanceMetadata {
            path,
            feature_files,
            benchmark_metrics,
            manual_evidence,
        } = metadata.expect("acceptance metadata should parse");
        assert_eq!(path, PathBuf::from("acceptance/step-01.yaml"));
        assert_eq!(
            feature_files,
            [PathBuf::from("acceptance/features/step-01.feature")]
        );
        assert!(benchmark_metrics.is_none());
        assert!(manual_evidence.auto.is_empty());
        assert!(manual_evidence.mixed.is_empty());
        assert!(manual_evidence.manual.is_empty());
    }

    #[test]
    fn load_acceptance_yaml_parses_mode_scoped_manual_evidence_paths() {
        let fixture_root = unique_fixture_root("acceptance-manual-evidence");
        fs::create_dir_all(fixture_root.join("acceptance"))
            .expect("fixture setup should create acceptance directory");

        fs::write(
            fixture_root.join("acceptance/step-13.yaml"),
            "slice_id: step-13\nmanual_evidence:\n  mixed:\n    - evidence/agent-review.json\n  manual:\n    - acceptance/evidence/human-review.json\n",
        )
        .expect("fixture setup should write acceptance yaml");

        let previous_dir = std::env::current_dir().expect("cwd should be readable");
        std::env::set_current_dir(&fixture_root).expect("cwd should switch to fixture root");

        let metadata = load_acceptance_yaml("step-13");

        std::env::set_current_dir(previous_dir).expect("cwd should restore to original location");

        let metadata = metadata.expect("acceptance metadata should parse");
        assert_eq!(
            metadata.manual_evidence.mixed,
            [PathBuf::from("acceptance/evidence/agent-review.json")]
        );
        assert_eq!(
            metadata.manual_evidence.manual,
            [PathBuf::from("acceptance/evidence/human-review.json")]
        );
    }

    fn unique_fixture_root(test_name: &str) -> PathBuf {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos();

        std::env::temp_dir().join(format!(
            "tonic-{test_name}-{timestamp}-{}",
            std::process::id()
        ))
    }
}
