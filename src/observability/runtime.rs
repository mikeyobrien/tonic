use super::schema::{
    ArtifactManifest, ArtifactRecord, LatestRunPointer, LegacySignals, PhaseRecord, Summary,
    SummaryArtifacts, TaskRunEntry, ToolInfo, ARTIFACT_SCHEMA_NAME, LATEST_SCHEMA_NAME,
    RUN_SCHEMA_NAME, SCHEMA_VERSION, TASK_SCHEMA_NAME,
};
use super::{
    ObservabilityError, OBS_DIR_ENV, OBS_ENABLE_ENV, OBS_PARENT_RUN_ID_ENV, OBS_RUN_ID_ENV,
    OBS_TASK_ID_ENV,
};
use rand::random;
use serde::Serialize;
use serde_json::json;
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[cfg_attr(not(test), allow(dead_code))]
#[derive(Debug)]
pub(crate) struct ObservabilityRun {
    run_id: String,
    task_id: Option<String>,
    parent_run_id: Option<String>,
    command: String,
    argv: Vec<String>,
    cwd: PathBuf,
    worktree_root: PathBuf,
    output_root: PathBuf,
    target_path: Option<String>,
    started_at: OffsetDateTime,
    started_at_instant: Instant,
    phases: Vec<PhaseRecord>,
    artifacts: Vec<ArtifactRecord>,
    error: Option<ObservabilityError>,
}

#[cfg_attr(not(test), allow(dead_code))]
impl ObservabilityRun {
    pub(crate) fn from_env(command: &str, argv: &[String], cwd: &Path) -> Option<Self> {
        if std::env::var(OBS_ENABLE_ENV).ok().as_deref() != Some("1") {
            return None;
        }

        let run_id = std::env::var(OBS_RUN_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty())
            .unwrap_or_else(generate_run_id);
        let task_id = std::env::var(OBS_TASK_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty());
        let parent_run_id = std::env::var(OBS_PARENT_RUN_ID_ENV)
            .ok()
            .filter(|value| !value.is_empty());
        let output_root = std::env::var_os(OBS_DIR_ENV)
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| cwd.join(".tonic/observability"));
        let target_path = argv
            .iter()
            .skip(1)
            .find(|arg| !arg.starts_with('-'))
            .cloned();

        Some(Self {
            run_id,
            task_id,
            parent_run_id,
            command: command.to_string(),
            argv: argv.to_vec(),
            cwd: cwd.to_path_buf(),
            worktree_root: cwd.to_path_buf(),
            output_root,
            target_path,
            started_at: OffsetDateTime::now_utc(),
            started_at_instant: Instant::now(),
            phases: Vec::new(),
            artifacts: Vec::new(),
            error: None,
        })
    }

    pub(crate) fn phase<T>(&mut self, name: &str, run: impl FnOnce() -> T) -> T {
        let started_at = Instant::now();
        let value = run();
        self.phases.push(PhaseRecord {
            name: name.to_string(),
            status: "ok".to_string(),
            elapsed_ms: started_at.elapsed().as_secs_f64() * 1000.0,
        });
        value
    }

    pub(crate) fn record_artifact(&mut self, kind: &str, path: &Path) {
        let bytes = fs::metadata(path).ok().map(|metadata| metadata.len());
        self.artifacts.push(ArtifactRecord {
            kind: kind.to_string(),
            path: path.display().to_string(),
            bytes,
        });
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn record_error(&mut self, error: ObservabilityError) {
        self.error = Some(error);
    }

    pub(crate) fn finish_ok(&mut self, exit_code: i32) -> Vec<String> {
        self.finish("ok", exit_code)
    }

    pub(crate) fn finish_error(
        &mut self,
        exit_code: i32,
        error: ObservabilityError,
    ) -> Vec<String> {
        self.error = Some(error);
        self.finish("error", exit_code)
    }

    fn finish(&self, status: &str, exit_code: i32) -> Vec<String> {
        let ended_at = OffsetDateTime::now_utc();
        let started_at = format_timestamp(self.started_at);
        let ended_at_str = format_timestamp(ended_at);
        let bundle_dir = self.output_root.join("runs").join(&self.run_id);
        let summary_path = bundle_dir.join("summary.json");
        let events_path = bundle_dir.join("events.jsonl");
        let artifacts_path = bundle_dir.join("artifacts.json");

        let summary = Summary {
            schema_name: RUN_SCHEMA_NAME,
            schema_version: SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            task_id: self.task_id.clone(),
            parent_run_id: self.parent_run_id.clone(),
            tool: ToolInfo {
                kind: "tonic-cli".to_string(),
                name: "tonic".to_string(),
                command: self.command.clone(),
            },
            cwd: self.cwd.display().to_string(),
            worktree_root: self.worktree_root.display().to_string(),
            argv: self.argv.clone(),
            target_path: self.target_path.clone(),
            status: status.to_string(),
            exit_code,
            started_at: started_at.clone(),
            ended_at: ended_at_str.clone(),
            duration_ms: self.started_at_instant.elapsed().as_secs_f64() * 1000.0,
            phases: self.phases.clone(),
            artifacts: SummaryArtifacts {
                bundle_dir: bundle_dir.display().to_string(),
                emitted: self.artifacts.clone(),
            },
            error: self.error.clone(),
            legacy_signals: LegacySignals::from_env(),
        };
        let artifacts = ArtifactManifest {
            schema_name: ARTIFACT_SCHEMA_NAME,
            schema_version: SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            items: self.artifacts.clone(),
        };
        let events = self.build_events(status, exit_code, &started_at, &ended_at_str);
        let latest = LatestRunPointer {
            schema_name: LATEST_SCHEMA_NAME,
            schema_version: SCHEMA_VERSION,
            run_id: self.run_id.clone(),
            status: status.to_string(),
            summary_path: summary_path.display().to_string(),
            ended_at: ended_at_str.clone(),
        };

        let mut warnings = Vec::new();
        if let Err(error) = fs::create_dir_all(&bundle_dir) {
            warnings.push(format!(
                "failed to create observability bundle directory {}: {error}",
                bundle_dir.display()
            ));
            return warnings;
        }

        if let Err(error) = write_json_file(&summary_path, &summary) {
            warnings.push(format!(
                "failed to write observability summary {}: {error}",
                summary_path.display()
            ));
        }
        if let Err(error) = write_json_lines(&events_path, &events) {
            warnings.push(format!(
                "failed to write observability events {}: {error}",
                events_path.display()
            ));
        }
        if let Err(error) = write_json_file(&artifacts_path, &artifacts) {
            warnings.push(format!(
                "failed to write observability artifacts {}: {error}",
                artifacts_path.display()
            ));
        }

        let latest_path = self.output_root.join("latest.json");
        if let Err(error) = write_json_file(&latest_path, &latest) {
            warnings.push(format!(
                "failed to write observability latest pointer {}: {error}",
                latest_path.display()
            ));
        }

        if let Some(task_id) = &self.task_id {
            let task_path = self
                .output_root
                .join("tasks")
                .join(task_id)
                .join("runs.jsonl");
            let task_entry = TaskRunEntry {
                schema_name: TASK_SCHEMA_NAME,
                schema_version: SCHEMA_VERSION,
                run_id: self.run_id.clone(),
                tool: "tonic-cli".to_string(),
                command: self.command.clone(),
                status: status.to_string(),
                started_at,
                ended_at: ended_at_str,
            };
            if let Err(error) = append_json_line(&task_path, &task_entry) {
                warnings.push(format!(
                    "failed to append observability task index {}: {error}",
                    task_path.display()
                ));
            }
        }

        warnings
    }

    fn build_events(
        &self,
        status: &str,
        exit_code: i32,
        started_at: &str,
        ended_at: &str,
    ) -> Vec<serde_json::Value> {
        let mut events = vec![json!({
            "type": "run.started",
            "run_id": &self.run_id,
            "task_id": &self.task_id,
            "parent_run_id": &self.parent_run_id,
            "command": &self.command,
            "argv": &self.argv,
            "cwd": self.cwd.display().to_string(),
            "at": started_at,
        })];

        for phase in &self.phases {
            events.push(json!({
                "type": "phase.finished",
                "run_id": &self.run_id,
                "phase": &phase.name,
                "status": &phase.status,
                "elapsed_ms": phase.elapsed_ms,
            }));
        }

        for artifact in &self.artifacts {
            events.push(json!({
                "type": "artifact.written",
                "run_id": &self.run_id,
                "kind": &artifact.kind,
                "path": &artifact.path,
                "bytes": artifact.bytes,
            }));
        }

        if let Some(error) = &self.error {
            events.push(json!({
                "type": "error.reported",
                "run_id": &self.run_id,
                "kind": &error.kind,
                "diagnostic_code": &error.diagnostic_code,
                "phase": &error.phase,
                "message": &error.message,
                "source": &error.source,
            }));
        }

        events.push(json!({
            "type": "run.finished",
            "run_id": &self.run_id,
            "status": status,
            "exit_code": exit_code,
            "ended_at": ended_at,
        }));
        events
    }
}

fn write_json_file(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let payload = serde_json::to_vec_pretty(value).map_err(io::Error::other)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, payload)?;
    Ok(())
}

fn write_json_lines(path: &Path, values: &[serde_json::Value]) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::File::create(path)?;
    for value in values {
        let payload = serde_json::to_string(value).map_err(io::Error::other)?;
        writeln!(file, "{payload}")?;
    }
    Ok(())
}

fn append_json_line(path: &Path, value: &impl Serialize) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = serde_json::to_string(value).map_err(io::Error::other)?;
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{payload}")?;
    Ok(())
}

fn generate_run_id() -> String {
    let now = OffsetDateTime::now_utc();
    format!(
        "run_{:04}{:02}{:02}_{:02}{:02}{:02}_{:06x}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second(),
        random::<u32>() & 0x00ff_ffff
    )
}

fn format_timestamp(timestamp: OffsetDateTime) -> String {
    timestamp
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}
