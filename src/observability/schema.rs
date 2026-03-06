use serde::Serialize;

pub(super) const RUN_SCHEMA_NAME: &str = "tonic.observability.run";
pub(super) const ARTIFACT_SCHEMA_NAME: &str = "tonic.observability.artifacts";
pub(super) const LATEST_SCHEMA_NAME: &str = "tonic.observability.latest";
pub(super) const TASK_SCHEMA_NAME: &str = "tonic.observability.task-run";
pub(super) const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct PhaseRecord {
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) elapsed_ms: f64,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ArtifactRecord {
    pub(crate) kind: String,
    pub(crate) path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ErrorSource {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) column: usize,
    pub(crate) offset: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ObservabilityError {
    pub(crate) kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) diagnostic_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) phase: Option<String>,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<ErrorSource>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ToolInfo {
    pub(super) kind: String,
    pub(super) name: String,
    pub(super) command: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SummaryArtifacts {
    pub(super) bundle_dir: String,
    pub(super) emitted: Vec<ArtifactRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LegacySignals {
    pub(super) profile_enabled: bool,
    pub(super) debug_cache: bool,
    pub(super) debug_module_loads: bool,
    pub(super) debug_types: bool,
    pub(super) memory_stats: bool,
    pub(super) memory_mode: Option<String>,
}

impl LegacySignals {
    pub(super) fn from_env() -> Self {
        Self {
            profile_enabled: std::env::var_os("TONIC_PROFILE_STDERR").is_some()
                || std::env::var_os("TONIC_PROFILE_OUT").is_some(),
            debug_cache: std::env::var_os("TONIC_DEBUG_CACHE").is_some(),
            debug_module_loads: std::env::var_os("TONIC_DEBUG_MODULE_LOADS").is_some(),
            debug_types: std::env::var_os("TONIC_DEBUG_TYPES").is_some(),
            memory_stats: std::env::var_os("TONIC_MEMORY_STATS").is_some(),
            memory_mode: std::env::var("TONIC_MEMORY_MODE")
                .ok()
                .filter(|value| !value.is_empty()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct Summary {
    pub(super) schema_name: &'static str,
    pub(super) schema_version: u32,
    pub(super) run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) parent_run_id: Option<String>,
    pub(super) tool: ToolInfo,
    pub(super) cwd: String,
    pub(super) worktree_root: String,
    pub(super) argv: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) target_path: Option<String>,
    pub(super) status: String,
    pub(super) exit_code: i32,
    pub(super) started_at: String,
    pub(super) ended_at: String,
    pub(super) duration_ms: f64,
    pub(super) phases: Vec<PhaseRecord>,
    pub(super) artifacts: SummaryArtifacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) error: Option<ObservabilityError>,
    pub(super) legacy_signals: LegacySignals,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ArtifactManifest {
    pub(super) schema_name: &'static str,
    pub(super) schema_version: u32,
    pub(super) run_id: String,
    pub(super) items: Vec<ArtifactRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct LatestRunPointer {
    pub(super) schema_name: &'static str,
    pub(super) schema_version: u32,
    pub(super) run_id: String,
    pub(super) status: String,
    pub(super) summary_path: String,
    pub(super) ended_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct TaskRunEntry {
    pub(super) schema_name: &'static str,
    pub(super) schema_version: u32,
    pub(super) run_id: String,
    pub(super) tool: String,
    pub(super) command: String,
    pub(super) status: String,
    pub(super) started_at: String,
    pub(super) ended_at: String,
}
