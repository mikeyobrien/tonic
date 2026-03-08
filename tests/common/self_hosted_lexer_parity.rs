use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::differential::CommandOutcome;

const SELF_HOSTED_LEXER_APP: &str = "examples/apps/self_hosted_lexer";
const CURATED_FIXTURES: &[&str] = &[
    "tests/fixtures/self_hosted_lexer_parity/keywords_module.tn",
    "tests/fixtures/self_hosted_lexer_parity/punctuation_call.tn",
    "tests/fixtures/self_hosted_lexer_parity/punctuation_list.tn",
    "tests/fixtures/self_hosted_lexer_parity/operators_arithmetic.tn",
    "tests/fixtures/self_hosted_lexer_parity/operators_compare.tn",
    "tests/fixtures/self_hosted_lexer_parity/numbers_comments_whitespace.tn",
    "tests/fixtures/self_hosted_lexer_parity/strings_heredoc.tn",
    "tests/fixtures/self_hosted_lexer_parity/interpolation.tn",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenDumpRecord {
    pub kind: String,
    pub lexeme: String,
    pub span_start: usize,
    pub span_end: usize,
}

#[derive(Debug, Clone)]
pub struct FixtureOutputs {
    pub fixture: String,
    pub source_path: PathBuf,
    pub reference: CommandOutcome,
    pub self_hosted: CommandOutcome,
    pub reference_tokens: Vec<TokenDumpRecord>,
    pub self_hosted_tokens: Vec<TokenDumpRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SelfHostedLexerMismatch {
    pub fixture: String,
    pub source_path: String,
    pub reason: String,
    pub reference: CommandOutcome,
    pub self_hosted: CommandOutcome,
    pub reference_tokens: Vec<TokenDumpRecord>,
    pub self_hosted_tokens: Vec<TokenDumpRecord>,
}

pub fn run_curated_corpus(
    tonic_bin: &Path,
    cwd: &Path,
) -> Result<(), Box<SelfHostedLexerMismatch>> {
    for fixture in CURATED_FIXTURES {
        let outputs = collect_fixture_outputs(tonic_bin, cwd, fixture)?;
        compare_fixture_outputs(&outputs)?;
    }

    Ok(())
}

pub fn collect_fixture_outputs(
    tonic_bin: &Path,
    cwd: &Path,
    fixture: &str,
) -> Result<FixtureOutputs, Box<SelfHostedLexerMismatch>> {
    let source_path = cwd.join(fixture);
    let reference = run_command(
        tonic_bin,
        cwd,
        &["check", fixture, "--dump-tokens", "--format", "json"],
        "reference",
    );
    let (self_hosted, self_hosted_log_path) = run_self_hosted_command(tonic_bin, cwd, fixture);

    let reference_tokens = parse_reference_tokens(&reference).map_err(|reason| {
        Box::new(mismatch(
            fixture,
            &source_path,
            reason,
            reference.clone(),
            self_hosted.clone(),
            Vec::new(),
            Vec::new(),
        ))
    })?;
    let self_hosted_tokens = parse_self_hosted_tokens(&self_hosted, &self_hosted_log_path)
        .map_err(|reason| {
            Box::new(mismatch(
                fixture,
                &source_path,
                reason,
                reference.clone(),
                self_hosted.clone(),
                reference_tokens.clone(),
                Vec::new(),
            ))
        })?;

    Ok(FixtureOutputs {
        fixture: fixture.to_string(),
        source_path,
        reference,
        self_hosted,
        reference_tokens,
        self_hosted_tokens,
    })
}

pub fn compare_fixture_outputs(
    outputs: &FixtureOutputs,
) -> Result<(), Box<SelfHostedLexerMismatch>> {
    if outputs.reference.exit_code != 0 {
        return Err(Box::new(mismatch(
            &outputs.fixture,
            &outputs.source_path,
            format!(
                "reference lexer command failed with exit {}",
                outputs.reference.exit_code
            ),
            outputs.reference.clone(),
            outputs.self_hosted.clone(),
            outputs.reference_tokens.clone(),
            outputs.self_hosted_tokens.clone(),
        )));
    }

    if outputs.self_hosted.exit_code != 0 {
        return Err(Box::new(mismatch(
            &outputs.fixture,
            &outputs.source_path,
            format!(
                "self-hosted lexer command failed with exit {}",
                outputs.self_hosted.exit_code
            ),
            outputs.reference.clone(),
            outputs.self_hosted.clone(),
            outputs.reference_tokens.clone(),
            outputs.self_hosted_tokens.clone(),
        )));
    }

    if outputs.reference_tokens == outputs.self_hosted_tokens {
        return Ok(());
    }

    Err(Box::new(mismatch(
        &outputs.fixture,
        &outputs.source_path,
        token_mismatch_reason(&outputs.reference_tokens, &outputs.self_hosted_tokens),
        outputs.reference.clone(),
        outputs.self_hosted.clone(),
        outputs.reference_tokens.clone(),
        outputs.self_hosted_tokens.clone(),
    )))
}

pub fn capture_mismatch_artifact(
    root: &Path,
    label: &str,
    source: &str,
    mismatch: &SelfHostedLexerMismatch,
) -> Result<PathBuf, String> {
    let artifact_dir = root
        .join("self-hosted-lexer-parity-artifacts")
        .join(sanitize_label(label));

    fs::create_dir_all(&artifact_dir).map_err(|error| {
        format!(
            "failed to create self-hosted lexer artifact directory {}: {}",
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

    write_json(
        &artifact_dir.join("reference.json"),
        &mismatch.reference_tokens,
        "reference token dump",
    )?;
    write_json(
        &artifact_dir.join("self_hosted.json"),
        &mismatch.self_hosted_tokens,
        "self-hosted token dump",
    )?;
    write_json(
        &artifact_dir.join("mismatch.json"),
        mismatch,
        "self-hosted lexer mismatch artifact",
    )?;

    Ok(artifact_dir)
}

fn write_json(path: &Path, value: &impl Serialize, label: &str) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(value)
        .map_err(|error| format!("failed to serialize {label}: {error}"))?;

    fs::write(path, payload)
        .map_err(|error| format!("failed to write {}: {}", path.display(), error))
}

fn mismatch(
    fixture: &str,
    source_path: &Path,
    reason: String,
    reference: CommandOutcome,
    self_hosted: CommandOutcome,
    reference_tokens: Vec<TokenDumpRecord>,
    self_hosted_tokens: Vec<TokenDumpRecord>,
) -> SelfHostedLexerMismatch {
    SelfHostedLexerMismatch {
        fixture: fixture.to_string(),
        source_path: source_path.display().to_string(),
        reason,
        reference,
        self_hosted,
        reference_tokens,
        self_hosted_tokens,
    }
}

fn token_mismatch_reason(
    reference_tokens: &[TokenDumpRecord],
    self_hosted_tokens: &[TokenDumpRecord],
) -> String {
    let min_len = reference_tokens.len().min(self_hosted_tokens.len());
    for idx in 0..min_len {
        if reference_tokens[idx] != self_hosted_tokens[idx] {
            return format!(
                "token mismatch at index {idx}: reference={:?}, self_hosted={:?}",
                reference_tokens[idx], self_hosted_tokens[idx]
            );
        }
    }

    format!(
        "token count mismatch: reference={}, self_hosted={}",
        reference_tokens.len(),
        self_hosted_tokens.len()
    )
}

fn parse_reference_tokens(command: &CommandOutcome) -> Result<Vec<TokenDumpRecord>, String> {
    if command.exit_code != 0 {
        return Ok(Vec::new());
    }

    serde_json::from_str(command.stdout.trim())
        .map_err(|error| format!("failed to parse reference token dump JSON from stdout: {error}"))
}

fn parse_self_hosted_tokens(
    command: &CommandOutcome,
    structured_log_path: &Path,
) -> Result<Vec<TokenDumpRecord>, String> {
    if command.exit_code != 0 {
        return Ok(Vec::new());
    }

    let log = fs::read_to_string(structured_log_path).map_err(|error| {
        format!(
            "failed to read self-hosted lexer structured log {}: {}",
            structured_log_path.display(),
            error
        )
    })?;

    let entry = log
        .lines()
        .find_map(|line| {
            let value: Value = serde_json::from_str(line).ok()?;
            if value.get("event") == Some(&Value::String("self_hosted_lexer.tokens".to_string())) {
                Some(value)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            format!(
                "self-hosted lexer structured log {} missing self_hosted_lexer.tokens event",
                structured_log_path.display()
            )
        })?;

    serde_json::from_value(entry["fields"]["tokens"].clone()).map_err(|error| {
        format!(
            "failed to parse self-hosted token dump JSON from {}: {}",
            structured_log_path.display(),
            error
        )
    })
}

fn run_self_hosted_command(
    tonic_bin: &Path,
    cwd: &Path,
    fixture: &str,
) -> (CommandOutcome, PathBuf) {
    let log_path = std::env::temp_dir().join(format!(
        "tonic-self-hosted-lexer-parity-{}-{}-{}.jsonl",
        sanitize_label(fixture),
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after unix epoch")
            .as_nanos()
    ));

    let args = ["run", SELF_HOSTED_LEXER_APP, fixture];
    (
        run_command_with_env(
            tonic_bin,
            cwd,
            &args,
            "self_hosted",
            &[("TONIC_SYSTEM_LOG_PATH", log_path.display().to_string())],
        ),
        log_path,
    )
}

fn run_command(tonic_bin: &Path, cwd: &Path, args: &[&str], phase: &str) -> CommandOutcome {
    run_command_with_env(tonic_bin, cwd, args, phase, &[])
}

fn run_command_with_env(
    tonic_bin: &Path,
    cwd: &Path,
    args: &[&str],
    phase: &str,
    envs: &[(&str, String)],
) -> CommandOutcome {
    let command: Vec<String> = std::iter::once(tonic_bin.display().to_string())
        .chain(args.iter().map(|value| value.to_string()))
        .collect();
    let mut process = Command::new(tonic_bin);
    process.current_dir(cwd).args(args);
    for (key, value) in envs {
        process.env(key, value);
    }

    match process.output() {
        Ok(output) => CommandOutcome {
            phase: phase.to_string(),
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            command,
        },
        Err(error) => CommandOutcome {
            phase: phase.to_string(),
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("failed to execute command: {error}"),
            command,
        },
    }
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
