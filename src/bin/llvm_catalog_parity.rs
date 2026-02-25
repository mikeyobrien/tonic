#[path = "llvm_catalog_parity/markdown.rs"]
mod markdown;

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

const DEFAULT_CATALOG: &str = "examples/parity/catalog.toml";
const DEFAULT_REPORT_JSON: &str = ".tonic/parity/llvm-catalog-parity.json";
const DEFAULT_REPORT_MD: &str = ".tonic/parity/llvm-catalog-parity.md";

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code as u8),
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<i32, String> {
    let cli = parse_args(env::args().skip(1).collect())?;

    if cli.help {
        print_help();
        return Ok(0);
    }

    let cwd = env::current_dir().map_err(|error| format!("failed to read current dir: {error}"))?;
    let tonic_bin = resolve_tonic_bin(cli.tonic_bin.as_deref(), &cwd)?;
    let report = run_catalog_parity(&cli, &cwd, &tonic_bin)?;

    write_report(
        &cli.report_json,
        &serde_json::to_string_pretty(&report)
            .map_err(|error| format!("failed to serialize json parity report: {error}"))?,
    )?;

    write_report(&cli.report_md, &markdown::render_markdown(&report))?;

    println!(
        "parity: compile {}/{} match, runtime {}/{} match",
        report.summary.compile_matches,
        report.summary.compile_total,
        report.summary.runtime_matches,
        report.summary.runtime_total
    );
    println!(
        "parity: reports written: {} {}",
        cli.report_json.display(),
        cli.report_md.display()
    );

    if cli.enforce && report.summary.total_mismatches > 0 {
        return Err(format!(
            "llvm parity enforce failed: {} mismatches (compile={}, runtime={})",
            report.summary.total_mismatches,
            report.summary.compile_mismatches,
            report.summary.runtime_mismatches
        ));
    }

    Ok(0)
}

#[derive(Debug, Clone)]
struct Cli {
    catalog: PathBuf,
    report_json: PathBuf,
    report_md: PathBuf,
    tonic_bin: Option<PathBuf>,
    enforce: bool,
    help: bool,
}

fn parse_args(args: Vec<String>) -> Result<Cli, String> {
    let mut cli = Cli {
        catalog: PathBuf::from(DEFAULT_CATALOG),
        report_json: PathBuf::from(DEFAULT_REPORT_JSON),
        report_md: PathBuf::from(DEFAULT_REPORT_MD),
        tonic_bin: None,
        enforce: false,
        help: false,
    };

    let mut idx = 0;
    while idx < args.len() {
        match args[idx].as_str() {
            "-h" | "--help" => {
                cli.help = true;
                idx += 1;
            }
            "--enforce" => {
                cli.enforce = true;
                idx += 1;
            }
            "--catalog" => {
                idx += 1;
                if idx >= args.len() {
                    return Err("--catalog requires a value".to_string());
                }
                cli.catalog = PathBuf::from(&args[idx]);
                idx += 1;
            }
            "--report-json" => {
                idx += 1;
                if idx >= args.len() {
                    return Err("--report-json requires a value".to_string());
                }
                cli.report_json = PathBuf::from(&args[idx]);
                idx += 1;
            }
            "--report-md" => {
                idx += 1;
                if idx >= args.len() {
                    return Err("--report-md requires a value".to_string());
                }
                cli.report_md = PathBuf::from(&args[idx]);
                idx += 1;
            }
            "--tonic-bin" => {
                idx += 1;
                if idx >= args.len() {
                    return Err("--tonic-bin requires a value".to_string());
                }
                cli.tonic_bin = Some(PathBuf::from(&args[idx]));
                idx += 1;
            }
            other => return Err(format!("unexpected argument '{other}'")),
        }
    }

    Ok(cli)
}

fn print_help() {
    println!(
        "Usage:\n  llvm_catalog_parity [--catalog <path>] [--report-json <path>] [--report-md <path>] [--tonic-bin <path>] [--enforce]"
    );
}

fn resolve_tonic_bin(explicit: Option<&Path>, cwd: &Path) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(path.to_path_buf());
    }

    if let Ok(path) = env::var("CARGO_BIN_EXE_tonic") {
        let as_path = PathBuf::from(path);
        if as_path.exists() {
            return Ok(as_path);
        }
    }

    if let Ok(path) = env::var("TONIC_BIN") {
        let as_path = PathBuf::from(path);
        if as_path.exists() {
            return Ok(as_path);
        }
    }

    if let Ok(current) = env::current_exe() {
        if let Some(parent) = current.parent() {
            let sibling = parent.join("tonic");
            if sibling.exists() {
                return Ok(sibling);
            }
        }
    }

    let fallback = cwd.join("target").join("debug").join("tonic");
    if fallback.exists() {
        return Ok(fallback);
    }

    let build = Command::new("cargo")
        .current_dir(cwd)
        .args(["build", "-q", "--bin", "tonic"])
        .output()
        .map_err(|error| format!("failed to execute cargo build --bin tonic: {error}"))?;

    if !build.status.success() {
        return Err(format!(
            "failed to build tonic binary (exit {}): {}",
            build.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&build.stderr)
        ));
    }

    if fallback.exists() {
        Ok(fallback)
    } else {
        Err(format!(
            "tonic binary not found at {} after cargo build --bin tonic",
            fallback.display()
        ))
    }
}

#[derive(Debug, Deserialize)]
struct Catalog {
    example: Vec<CatalogEntry>,
}

#[derive(Debug, Deserialize)]
struct CatalogEntry {
    path: String,
    check_exit: i32,
    run_exit: i32,
    stdout: Option<String>,
    stderr_contains: Option<String>,
    status: String,
}

#[derive(Debug, Serialize)]
struct ParityReport {
    catalog: String,
    tonic_bin: String,
    summary: Summary,
    fixtures: Vec<FixtureReport>,
    top_failure_causes: Vec<FailureCause>,
}

#[derive(Debug, Serialize)]
struct Summary {
    active_total: usize,
    compile_total: usize,
    compile_matches: usize,
    compile_mismatches: usize,
    runtime_total: usize,
    runtime_matches: usize,
    runtime_mismatches: usize,
    total_mismatches: usize,
}

#[derive(Debug, Serialize)]
struct FailureCause {
    reason: String,
    count: usize,
    fixtures: Vec<String>,
}

#[derive(Debug, Serialize)]
struct FixtureReport {
    path: String,
    compile: CommandOutcome,
    runtime: Option<CommandOutcome>,
    mismatches: Vec<FixtureMismatch>,
}

#[derive(Debug, Serialize)]
struct FixtureMismatch {
    phase: String,
    reason: String,
}

#[derive(Debug, Clone, Serialize)]
struct CommandOutcome {
    phase: String,
    command: Vec<String>,
    exit_code: i32,
    stdout: String,
    stderr: String,
}

fn run_catalog_parity(cli: &Cli, cwd: &Path, tonic_bin: &Path) -> Result<ParityReport, String> {
    let catalog_raw = fs::read_to_string(&cli.catalog)
        .map_err(|error| format!("failed to read {}: {error}", cli.catalog.display()))?;
    let catalog: Catalog = toml::from_str(&catalog_raw)
        .map_err(|error| format!("failed to parse {}: {error}", cli.catalog.display()))?;

    let active: Vec<CatalogEntry> = catalog
        .example
        .into_iter()
        .filter(|entry| entry.status == "active")
        .collect();

    let mut fixtures = Vec::with_capacity(active.len());
    let mut compile_matches = 0usize;
    let mut compile_mismatches = 0usize;
    let mut runtime_total = 0usize;
    let mut runtime_matches = 0usize;
    let mut runtime_mismatches = 0usize;
    let mut grouped_causes: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for entry in active {
        let compile = run_command(tonic_bin, cwd, &["compile", &entry.path], "compile");

        let mut mismatches = Vec::new();

        if compile.exit_code == entry.check_exit {
            compile_matches += 1;
        } else {
            compile_mismatches += 1;
            mismatches.push(FixtureMismatch {
                phase: "compile".to_string(),
                reason: format!(
                    "compile exit mismatch: expected {}, got {}",
                    entry.check_exit, compile.exit_code
                ),
            });
        }

        let runtime = if compile.exit_code == 0 {
            runtime_total += 1;
            match parse_compile_artifact_path(&compile.stdout) {
                Some(artifact) => {
                    let output = run_executable(cwd, &artifact);
                    let runtime_issues = evaluate_runtime(&entry, &output);
                    if runtime_issues.is_empty() {
                        runtime_matches += 1;
                    } else {
                        runtime_mismatches += 1;
                        mismatches.extend(runtime_issues);
                    }
                    Some(output)
                }
                None => {
                    runtime_mismatches += 1;
                    mismatches.push(FixtureMismatch {
                        phase: "runtime".to_string(),
                        reason: "compile succeeded but artifact path missing from stdout"
                            .to_string(),
                    });
                    None
                }
            }
        } else {
            None
        };

        for mismatch in &mismatches {
            grouped_causes
                .entry(mismatch.reason.clone())
                .or_default()
                .insert(entry.path.clone());
        }

        fixtures.push(FixtureReport {
            path: entry.path,
            compile,
            runtime,
            mismatches,
        });
    }

    let mut top_failure_causes: Vec<FailureCause> = grouped_causes
        .into_iter()
        .map(|(reason, fixtures)| FailureCause {
            count: fixtures.len(),
            reason,
            fixtures: fixtures.into_iter().collect(),
        })
        .collect();

    top_failure_causes.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.reason.cmp(&right.reason))
    });

    let summary = Summary {
        active_total: fixtures.len(),
        compile_total: fixtures.len(),
        compile_matches,
        compile_mismatches,
        runtime_total,
        runtime_matches,
        runtime_mismatches,
        total_mismatches: compile_mismatches + runtime_mismatches,
    };

    Ok(ParityReport {
        catalog: cli.catalog.display().to_string(),
        tonic_bin: tonic_bin.display().to_string(),
        summary,
        fixtures,
        top_failure_causes,
    })
}

fn run_command(tonic_bin: &Path, cwd: &Path, args: &[&str], phase: &str) -> CommandOutcome {
    let command: Vec<String> = std::iter::once(tonic_bin.display().to_string())
        .chain(args.iter().map(|value| value.to_string()))
        .collect();

    match Command::new(tonic_bin).current_dir(cwd).args(args).output() {
        Ok(output) => CommandOutcome {
            phase: phase.to_string(),
            command,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => CommandOutcome {
            phase: phase.to_string(),
            command,
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("failed to execute command: {error}"),
        },
    }
}

fn run_executable(cwd: &Path, artifact: &str) -> CommandOutcome {
    let path = Path::new(artifact);
    let executable = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    let command = vec![executable.display().to_string()];

    match Command::new(&executable).current_dir(cwd).output() {
        Ok(output) => CommandOutcome {
            phase: "runtime".to_string(),
            command,
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        },
        Err(error) => CommandOutcome {
            phase: "runtime".to_string(),
            command,
            exit_code: -1,
            stdout: String::new(),
            stderr: format!("failed to execute compiled artifact: {error}"),
        },
    }
}

fn evaluate_runtime(entry: &CatalogEntry, runtime: &CommandOutcome) -> Vec<FixtureMismatch> {
    let mut mismatches = Vec::new();

    if runtime.exit_code != entry.run_exit {
        mismatches.push(FixtureMismatch {
            phase: "runtime".to_string(),
            reason: format!(
                "runtime exit mismatch: expected {}, got {}",
                entry.run_exit, runtime.exit_code
            ),
        });
    }

    if let Some(expected_stdout) = &entry.stdout {
        if runtime.stdout != *expected_stdout {
            mismatches.push(FixtureMismatch {
                phase: "runtime".to_string(),
                reason: "runtime stdout mismatch".to_string(),
            });
        }
    }

    if let Some(expected_stderr) = &entry.stderr_contains {
        if !runtime.stderr.contains(expected_stderr) {
            mismatches.push(FixtureMismatch {
                phase: "runtime".to_string(),
                reason: format!("runtime stderr missing expected substring: {expected_stderr}"),
            });
        }
    }

    mismatches
}

fn parse_compile_artifact_path(stdout: &str) -> Option<String> {
    stdout
        .lines()
        .rev()
        .find_map(|line| line.strip_prefix("compile: ok ").map(str::trim))
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
}

fn write_report(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| {
            format!(
                "failed to create report directory {}: {error}",
                parent.display()
            )
        })?;
    }

    fs::write(path, content).map_err(|error| format!("failed to write {}: {error}", path.display()))
}
