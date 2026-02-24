use serde::Serialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct CommandOutcome {
    pub phase: String,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub command: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DifferentialMismatch {
    pub fixture: String,
    pub reason: String,
    pub interpreter: CommandOutcome,
    pub native: CommandOutcome,
}

pub fn run_differential_fixture(
    tonic_bin: &Path,
    cwd: &Path,
    fixture: &str,
) -> Result<(), Box<DifferentialMismatch>> {
    let interpreter = run_command(
        tonic_bin,
        cwd,
        &["run".to_string(), fixture.to_string()],
        "run",
    )
    .unwrap_or_else(|message| synthetic_failure("run", fixture, message));

    let native = run_native_path(tonic_bin, cwd, fixture)
        .unwrap_or_else(|message| synthetic_failure("compile", fixture, message));

    if outcomes_match(&interpreter, &native) {
        return Ok(());
    }

    Err(Box::new(DifferentialMismatch {
        fixture: fixture.to_string(),
        reason: mismatch_reason(&interpreter, &native),
        interpreter,
        native,
    }))
}

pub fn minimize_source_by_lines<F>(source: &str, mut still_fails: F) -> String
where
    F: FnMut(&str) -> bool,
{
    if source.trim().is_empty() || !still_fails(source) {
        return source.to_string();
    }

    let trailing_newline = source.ends_with('\n');
    let mut lines: Vec<String> = source.lines().map(ToOwned::to_owned).collect();

    if lines.len() <= 1 {
        return source.to_string();
    }

    let mut partitions = 2usize;

    while lines.len() > 1 {
        let chunk_size = lines.len().div_ceil(partitions);
        let mut reduced = false;
        let mut start = 0usize;

        while start < lines.len() {
            let end = (start + chunk_size).min(lines.len());
            let mut candidate = lines.clone();
            candidate.drain(start..end);

            if candidate.is_empty() {
                start = end;
                continue;
            }

            let candidate_source = join_lines(&candidate, trailing_newline);
            if still_fails(&candidate_source) {
                lines = candidate;
                partitions = 2;
                reduced = true;
                break;
            }

            start = end;
        }

        if !reduced {
            if partitions >= lines.len() {
                break;
            }
            partitions = (partitions * 2).min(lines.len());
        }
    }

    join_lines(&lines, trailing_newline)
}

pub fn capture_mismatch_artifact(
    root: &Path,
    label: &str,
    source: &str,
    minimized_source: &str,
    mismatch: &DifferentialMismatch,
) -> Result<std::path::PathBuf, String> {
    let artifact_dir = root
        .join("differential-artifacts")
        .join(sanitize_label(label));

    fs::create_dir_all(&artifact_dir).map_err(|error| {
        format!(
            "failed to create differential artifact directory {}: {}",
            artifact_dir.display(),
            error
        )
    })?;

    fs::write(artifact_dir.join("program.tn"), source).map_err(|error| {
        format!(
            "failed to write source artifact {}: {}",
            artifact_dir.join("program.tn").display(),
            error
        )
    })?;

    fs::write(artifact_dir.join("program.min.tn"), minimized_source).map_err(|error| {
        format!(
            "failed to write minimized source artifact {}: {}",
            artifact_dir.join("program.min.tn").display(),
            error
        )
    })?;

    let artifact = MismatchArtifact {
        fixture: mismatch.fixture.clone(),
        reason: mismatch.reason.clone(),
        interpreter: mismatch.interpreter.clone(),
        native: mismatch.native.clone(),
        replay_commands: vec![
            mismatch.interpreter.command.join(" "),
            mismatch.native.command.join(" "),
        ],
    };

    let payload = serde_json::to_string_pretty(&artifact)
        .map_err(|error| format!("failed to serialize mismatch artifact payload: {error}"))?;

    fs::write(artifact_dir.join("mismatch.json"), payload).map_err(|error| {
        format!(
            "failed to write mismatch artifact payload {}: {}",
            artifact_dir.join("mismatch.json").display(),
            error
        )
    })?;

    Ok(artifact_dir)
}

#[derive(Debug, Clone, Serialize)]
struct MismatchArtifact {
    fixture: String,
    reason: String,
    interpreter: CommandOutcome,
    native: CommandOutcome,
    replay_commands: Vec<String>,
}

fn run_native_path(tonic_bin: &Path, cwd: &Path, fixture: &str) -> Result<CommandOutcome, String> {
    let compile = run_command(
        tonic_bin,
        cwd,
        &[
            "compile".to_string(),
            fixture.to_string(),
            "--backend".to_string(),
            "llvm".to_string(),
        ],
        "compile",
    )?;

    if compile.exit_code != 0 {
        return Ok(compile);
    }

    let manifest_path = native_manifest_path(fixture);
    run_command(tonic_bin, cwd, &["run".to_string(), manifest_path], "run")
}

fn native_manifest_path(fixture: &str) -> String {
    let stem = Path::new(fixture)
        .file_stem()
        .map(|value| value.to_string_lossy().into_owned())
        .unwrap_or_else(|| "out".to_string());

    format!(".tonic/build/{stem}.tnx.json")
}

fn run_command(
    tonic_bin: &Path,
    cwd: &Path,
    args: &[String],
    phase: &str,
) -> Result<CommandOutcome, String> {
    let output = std::process::Command::new(tonic_bin)
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|error| {
            format!(
                "failed to execute {} {}: {error}",
                tonic_bin.display(),
                args.join(" ")
            )
        })?;

    Ok(CommandOutcome {
        phase: phase.to_string(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        command: std::iter::once(tonic_bin.display().to_string())
            .chain(args.iter().cloned())
            .collect(),
    })
}

fn synthetic_failure(phase: &str, fixture: &str, message: String) -> CommandOutcome {
    CommandOutcome {
        phase: phase.to_string(),
        exit_code: -1,
        stdout: String::new(),
        stderr: message,
        command: vec!["tonic".to_string(), phase.to_string(), fixture.to_string()],
    }
}

fn outcomes_match(interpreter: &CommandOutcome, native: &CommandOutcome) -> bool {
    native.phase == "run"
        && interpreter.exit_code == native.exit_code
        && interpreter.stdout == native.stdout
        && interpreter.stderr == native.stderr
}

fn mismatch_reason(interpreter: &CommandOutcome, native: &CommandOutcome) -> String {
    if native.phase != "run" {
        return format!(
            "native {} failed with exit {}",
            native.phase, native.exit_code
        );
    }

    if interpreter.exit_code != native.exit_code {
        return format!(
            "exit code mismatch: interpreter={}, native={}",
            interpreter.exit_code, native.exit_code
        );
    }

    if interpreter.stdout != native.stdout {
        return "stdout mismatch".to_string();
    }

    if interpreter.stderr != native.stderr {
        return "stderr mismatch".to_string();
    }

    "backend output mismatch".to_string()
}

fn join_lines(lines: &[String], trailing_newline: bool) -> String {
    let mut source = lines.join("\n");
    if trailing_newline {
        source.push('\n');
    }
    source
}

fn sanitize_label(label: &str) -> String {
    let mut sanitized = String::with_capacity(label.len());
    for ch in label.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' {
            sanitized.push(ch);
        } else {
            sanitized.push('-');
        }
    }

    if sanitized.is_empty() {
        "mismatch".to_string()
    } else {
        sanitized
    }
}
