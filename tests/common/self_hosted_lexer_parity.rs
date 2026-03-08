use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::differential::CommandOutcome;

const SELF_HOSTED_LEXER_APP: &str = "examples/apps/self_hosted_lexer";
const CURATED_FIXTURES: &[&str] = &["tests/fixtures/self_hosted_lexer_parity/keywords_module.tn"];

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
    let self_hosted = run_command(
        tonic_bin,
        cwd,
        &["run", SELF_HOSTED_LEXER_APP, fixture],
        "self_hosted",
    );

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
    let self_hosted_tokens = parse_self_hosted_tokens(&self_hosted).map_err(|reason| {
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

fn parse_self_hosted_tokens(command: &CommandOutcome) -> Result<Vec<TokenDumpRecord>, String> {
    if command.exit_code != 0 {
        return Ok(Vec::new());
    }

    let marker = ":tokens => ";
    let start = command
        .stdout
        .find(marker)
        .ok_or_else(|| "self-hosted lexer output missing ':tokens => ' marker".to_string())?
        + marker.len();

    let mut parser = TokenListParser::new(&command.stdout[start..]);
    let tokens = parser.parse_token_list()?;
    parser.skip_ws();
    Ok(tokens)
}

fn run_command(tonic_bin: &Path, cwd: &Path, args: &[&str], phase: &str) -> CommandOutcome {
    let command: Vec<String> = std::iter::once(tonic_bin.display().to_string())
        .chain(args.iter().map(|value| value.to_string()))
        .collect();

    match Command::new(tonic_bin).current_dir(cwd).args(args).output() {
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

struct TokenListParser<'a> {
    input: &'a str,
    index: usize,
}

impl<'a> TokenListParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, index: 0 }
    }

    fn parse_token_list(&mut self) -> Result<Vec<TokenDumpRecord>, String> {
        self.skip_ws();
        self.expect_byte(b'[')?;
        self.skip_ws();

        let mut tokens = Vec::new();
        if self.peek_byte() == Some(b']') {
            self.index += 1;
            return Ok(tokens);
        }

        loop {
            tokens.push(self.parse_token_map()?);
            self.skip_ws();
            match self.peek_byte() {
                Some(b',') => {
                    self.index += 1;
                    self.skip_ws();
                }
                Some(b']') => {
                    self.index += 1;
                    break;
                }
                other => {
                    return Err(format!(
                        "expected ',' or ']' while parsing token list, found {:?}",
                        other.map(char::from)
                    ));
                }
            }
        }

        Ok(tokens)
    }

    fn parse_token_map(&mut self) -> Result<TokenDumpRecord, String> {
        self.skip_ws();
        self.expect_byte(b'%')?;
        self.expect_byte(b'{')?;

        let mut kind = None;
        let mut lexeme = None;
        let mut span_start = None;
        let mut span_end = None;

        loop {
            self.skip_ws();
            if self.peek_byte() == Some(b'}') {
                self.index += 1;
                break;
            }

            let key = self.parse_atom_key()?;
            self.skip_ws();
            self.expect_byte(b'=')?;
            self.expect_byte(b'>')?;
            self.skip_ws();

            match key.as_str() {
                "kind" => kind = Some(self.parse_string()?),
                "lexeme" => lexeme = Some(self.parse_string()?),
                "span_start" => span_start = Some(self.parse_usize()?),
                "span_end" => span_end = Some(self.parse_usize()?),
                _ => self.skip_value()?,
            }

            self.skip_ws();
            match self.peek_byte() {
                Some(b',') => {
                    self.index += 1;
                }
                Some(b'}') => {
                    self.index += 1;
                    break;
                }
                other => {
                    return Err(format!(
                        "expected ',' or '}}' while parsing token map, found {:?}",
                        other.map(char::from)
                    ));
                }
            }
        }

        Ok(TokenDumpRecord {
            kind: kind.ok_or_else(|| "token map missing :kind".to_string())?,
            lexeme: lexeme.ok_or_else(|| "token map missing :lexeme".to_string())?,
            span_start: span_start.ok_or_else(|| "token map missing :span_start".to_string())?,
            span_end: span_end.ok_or_else(|| "token map missing :span_end".to_string())?,
        })
    }

    fn parse_atom_key(&mut self) -> Result<String, String> {
        self.expect_byte(b':')?;
        let start = self.index;
        while let Some(byte) = self.peek_byte() {
            if byte.is_ascii_alphanumeric() || byte == b'_' {
                self.index += 1;
            } else {
                break;
            }
        }

        if self.index == start {
            return Err("expected atom key after ':'".to_string());
        }

        Ok(self.input[start..self.index].to_string())
    }

    fn parse_string(&mut self) -> Result<String, String> {
        self.expect_byte(b'"')?;
        let mut value = Vec::new();

        loop {
            let byte = self.next_byte().ok_or_else(|| {
                "unterminated string while parsing self-hosted lexer output".to_string()
            })?;
            match byte {
                b'"' => break,
                b'\\' => {
                    let escaped = self.next_byte().ok_or_else(|| {
                        "unterminated escape while parsing self-hosted lexer output".to_string()
                    })?;
                    match escaped {
                        b'n' => value.push(b'\n'),
                        b'r' => value.push(b'\r'),
                        b't' => value.push(b'\t'),
                        b'"' => value.push(b'"'),
                        b'\\' => value.push(b'\\'),
                        other => value.push(other),
                    }
                }
                other => value.push(other),
            }
        }

        String::from_utf8(value).map_err(|error| {
            format!("failed to decode UTF-8 string from self-hosted lexer output: {error}")
        })
    }

    fn parse_usize(&mut self) -> Result<usize, String> {
        let start = self.index;
        while let Some(byte) = self.peek_byte() {
            if byte.is_ascii_digit() {
                self.index += 1;
            } else {
                break;
            }
        }

        if self.index == start {
            return Err(
                "expected integer value while parsing self-hosted lexer output".to_string(),
            );
        }

        self.input[start..self.index]
            .parse::<usize>()
            .map_err(|error| format!("failed to parse integer token field: {error}"))
    }

    fn skip_value(&mut self) -> Result<(), String> {
        match self.peek_byte() {
            Some(b'"') => {
                self.parse_string()?;
                Ok(())
            }
            Some(byte) if byte.is_ascii_digit() => {
                self.parse_usize()?;
                Ok(())
            }
            Some(b':') => {
                self.parse_atom_key()?;
                Ok(())
            }
            other => Err(format!(
                "unsupported self-hosted lexer value while parsing token map: {:?}",
                other.map(char::from)
            )),
        }
    }

    fn expect_byte(&mut self, expected: u8) -> Result<(), String> {
        match self.next_byte() {
            Some(actual) if actual == expected => Ok(()),
            Some(actual) => Err(format!(
                "expected '{}', found '{}' while parsing self-hosted lexer output",
                char::from(expected),
                char::from(actual)
            )),
            None => Err(format!(
                "expected '{}', found end of input while parsing self-hosted lexer output",
                char::from(expected)
            )),
        }
    }

    fn skip_ws(&mut self) {
        while let Some(byte) = self.peek_byte() {
            if byte.is_ascii_whitespace() {
                self.index += 1;
            } else {
                break;
            }
        }
    }

    fn peek_byte(&self) -> Option<u8> {
        self.input.as_bytes().get(self.index).copied()
    }

    fn next_byte(&mut self) -> Option<u8> {
        let byte = self.peek_byte()?;
        self.index += 1;
        Some(byte)
    }
}
