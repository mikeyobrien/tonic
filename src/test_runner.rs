use crate::ir::{lower_ast_to_ir, IrProgram};
use crate::lexer::scan_tokens;
use crate::manifest::inject_optional_stdlib;
use crate::parser::parse_ast;
use crate::resolver::resolve_ast;
use crate::runtime::{evaluate_named_function, RuntimeValue};
use crate::typing::infer_types;
use serde_json::json;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestOutputFormat {
    Text,
    Json,
}

impl TestOutputFormat {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "text" => Some(Self::Text),
            "json" => Some(Self::Json),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCaseResult {
    pub id: String,
    pub status: TestCaseStatus,
    pub error: Option<String>,
    pub duration: Duration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestCaseStatus {
    Passed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub duration: Duration,
    pub results: Vec<TestCaseResult>,
}

impl TestRunReport {
    pub fn succeeded(&self) -> bool {
        self.failed == 0
    }

    pub fn render_text(&self) -> Vec<String> {
        let mut lines = Vec::new();

        for result in &self.results {
            let duration = format_duration(result.duration);
            match result.status {
                TestCaseStatus::Passed => {
                    lines.push(format!("test {} ... ok ({duration})", result.id));
                }
                TestCaseStatus::Failed => {
                    lines.push(format!("test {} ... FAILED ({duration})", result.id));
                    if let Some(error) = &result.error {
                        lines.push(format!("  error: {error}"));
                    }
                }
            }
        }

        // Failure summary: re-list failed tests with full errors at the end
        let failures: Vec<&TestCaseResult> = self
            .results
            .iter()
            .filter(|r| r.status == TestCaseStatus::Failed)
            .collect();

        if !failures.is_empty() {
            lines.push(String::new());
            lines.push("Failures:".to_string());
            lines.push(String::new());
            for (i, result) in failures.iter().enumerate() {
                lines.push(format!("  {}. {}", i + 1, result.id));
                if let Some(error) = &result.error {
                    for error_line in error.lines() {
                        lines.push(format!("     {error_line}"));
                    }
                }
                lines.push(String::new());
            }
        }

        let status = if self.succeeded() { "ok" } else { "FAILED" };
        let total_duration = format_duration(self.duration);
        lines.push(format!(
            "test result: {status}. {} passed; {} failed; {} total; finished in {total_duration}",
            self.passed, self.failed, self.total
        ));

        lines
    }

    pub fn render_json(&self) -> serde_json::Value {
        let result_to_json = |result: &TestCaseResult| {
            json!({
                "id": result.id,
                "status": match result.status {
                    TestCaseStatus::Passed => "passed",
                    TestCaseStatus::Failed => "failed",
                },
                "error": result.error,
                "duration_ms": duration_ms(result.duration),
            })
        };

        json!({
            "status": if self.succeeded() { "ok" } else { "failed" },
            "total": self.total,
            "passed": self.passed,
            "failed": self.failed,
            "duration_ms": duration_ms(self.duration),
            "results": self.results.iter().map(result_to_json).collect::<Vec<_>>(),
            "failures": self.results.iter()
                .filter(|r| r.status == TestCaseStatus::Failed)
                .map(result_to_json)
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestRunnerError {
    Failure(String),
    SourceDiagnostic {
        message: String,
        filename: Option<String>,
        source: String,
        offset: Option<usize>,
    },
}

struct TestSuite {
    tests: Vec<String>,
    ir: IrProgram,
}

pub fn list_tests(path: &str, filter: Option<&str>) -> Result<Vec<String>, TestRunnerError> {
    let target = Path::new(path);
    let test_files = discover_test_files(target)?;
    let mut all_tests = Vec::new();

    for file in test_files {
        let source = std::fs::read_to_string(&file).map_err(|error| {
            TestRunnerError::Failure(format!(
                "failed to read source file {}: {error}",
                file.display()
            ))
        })?;

        let suite = compile_suite(&file, &source)?;

        for test_name in suite.tests {
            if let Some(pattern) = filter {
                if !test_name.contains(pattern) {
                    continue;
                }
            }
            all_tests.push(test_name);
        }
    }

    Ok(all_tests)
}

pub fn run(
    path: &str,
    filter: Option<&str>,
    fail_fast: bool,
) -> Result<TestRunReport, TestRunnerError> {
    let target = Path::new(path);
    let test_files = discover_test_files(target)?;
    let mut results = Vec::new();
    let run_start = Instant::now();

    'outer: for file in test_files {
        let source = std::fs::read_to_string(&file).map_err(|error| {
            TestRunnerError::Failure(format!(
                "failed to read source file {}: {error}",
                file.display()
            ))
        })?;

        let suite = compile_suite(&file, &source)?;

        for test_name in suite.tests {
            if let Some(pattern) = filter {
                if !test_name.contains(pattern) {
                    continue;
                }
            }
            let test_start = Instant::now();
            match evaluate_named_function(&suite.ir, &test_name) {
                Ok(RuntimeValue::ResultErr(reason)) => {
                    let error = format_assertion_failure(&reason);
                    results.push(TestCaseResult {
                        id: test_name,
                        status: TestCaseStatus::Failed,
                        error: Some(error),
                        duration: test_start.elapsed(),
                    });
                    if fail_fast {
                        break 'outer;
                    }
                }
                Ok(_) => {
                    results.push(TestCaseResult {
                        id: test_name,
                        status: TestCaseStatus::Passed,
                        error: None,
                        duration: test_start.elapsed(),
                    });
                }
                Err(error) => {
                    results.push(TestCaseResult {
                        id: test_name,
                        status: TestCaseStatus::Failed,
                        error: Some(error.to_string()),
                        duration: test_start.elapsed(),
                    });
                    if fail_fast {
                        break 'outer;
                    }
                }
            }
        }
    }

    let passed = results
        .iter()
        .filter(|result| result.status == TestCaseStatus::Passed)
        .count();
    let failed = results.len().saturating_sub(passed);

    Ok(TestRunReport {
        total: results.len(),
        passed,
        failed,
        duration: run_start.elapsed(),
        results,
    })
}

fn format_duration(d: Duration) -> String {
    let ms = d.as_secs_f64() * 1000.0;
    if ms < 1.0 {
        format!("{:.2}ms", ms)
    } else if ms < 1000.0 {
        format!("{:.1}ms", ms)
    } else {
        format!("{:.2}s", d.as_secs_f64())
    }
}

fn duration_ms(d: Duration) -> f64 {
    (d.as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

fn discover_test_files(path: &Path) -> Result<Vec<PathBuf>, TestRunnerError> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    if !path.is_dir() {
        return Err(TestRunnerError::Failure(format!(
            "failed to read source file {}: not a file or directory",
            path.display()
        )));
    }

    let mut files = Vec::new();
    let mut pending = vec![path.to_path_buf()];

    while let Some(directory) = pending.pop() {
        let entries = std::fs::read_dir(&directory).map_err(|error| {
            TestRunnerError::Failure(format!(
                "failed to read source directory {}: {error}",
                directory.display()
            ))
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                TestRunnerError::Failure(format!(
                    "failed to read source directory {}: {error}",
                    directory.display()
                ))
            })?;

            let entry_path = entry.path();
            if entry_path.is_dir() {
                let skip_hidden = entry_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('.'));
                let skip_target = entry_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name == "target");

                if !skip_hidden && !skip_target {
                    pending.push(entry_path);
                }
                continue;
            }

            if is_test_file_name(&entry_path) {
                files.push(entry_path);
            }
        }
    }

    files.sort();
    Ok(files)
}

fn is_test_file_name(path: &Path) -> bool {
    if path.extension().and_then(|extension| extension.to_str()) != Some("tn") {
        return false;
    }

    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    file_name.starts_with("test_") || file_name.ends_with("_test.tn")
}

/// Format a structured assertion failure from the Assert module into a human-readable message.
/// Recognizes the `{:assertion_failed, details}` tuple convention used by Assert host functions.
fn format_assertion_failure(reason: &RuntimeValue) -> String {
    match reason {
        // assert/refute: {:assertion_failed, {:assert|:refute, message}}
        RuntimeValue::Tuple(tag, details) if matches!(**tag, RuntimeValue::Atom(ref a) if a == "assertion_failed") =>
        {
            match details.as_ref() {
                // Simple assert/refute: {type_atom, message_string}
                RuntimeValue::Tuple(type_atom, message) => {
                    let kind = match type_atom.as_ref() {
                        RuntimeValue::Atom(a) => a.as_str(),
                        _ => "assert",
                    };
                    let msg = match message.as_ref() {
                        RuntimeValue::String(s) => s.clone(),
                        other => other.render(),
                    };
                    format!("{kind} failed: {msg}")
                }
                // assert_equal/assert_not_equal: keyword-list-style details
                RuntimeValue::List(entries) => {
                    let mut kind = "assert_equal";
                    let mut left = None;
                    let mut right = None;
                    let mut container = None;
                    let mut element = None;
                    let mut delta = None;
                    let mut message = None;

                    for entry in entries {
                        if let RuntimeValue::Tuple(key, val) = entry {
                            match key.as_ref() {
                                RuntimeValue::Atom(k) if k == "type" => {
                                    if let RuntimeValue::Atom(t) = val.as_ref() {
                                        kind = match t.as_str() {
                                            "assert_not_equal" => "assert_not_equal",
                                            "assert_contains" => "assert_contains",
                                            "assert_in_delta" => "assert_in_delta",
                                            _ => "assert_equal",
                                        };
                                    }
                                }
                                RuntimeValue::Atom(k) if k == "left" => {
                                    left = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "right" => {
                                    right = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "container" => {
                                    container = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "element" => {
                                    element = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "delta" => {
                                    delta = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "message" => {
                                    if let RuntimeValue::String(s) = val.as_ref() {
                                        message = Some(s.clone());
                                    }
                                }
                                _ => {}
                            }
                        }
                    }

                    let mut lines = vec![format!("{kind} failed: {}", message.unwrap_or_default())];
                    match kind {
                        "assert_contains" => {
                            if let Some(c) = container {
                                lines.push(format!("  container: {c}"));
                            }
                            if let Some(e) = element {
                                lines.push(format!("  element:   {e}"));
                            }
                        }
                        "assert_in_delta" => {
                            if let Some(l) = left {
                                lines.push(format!("  left:  {l}"));
                            }
                            if let Some(r) = right {
                                lines.push(format!("  right: {r}"));
                            }
                            if let Some(d) = delta {
                                lines.push(format!("  delta: {d}"));
                            }
                        }
                        _ => {
                            if let Some(l) = left {
                                lines.push(format!("  left:  {l}"));
                            }
                            if let Some(r) = right {
                                lines.push(format!("  right: {r}"));
                            }
                        }
                    }
                    lines.join("\n")
                }
                other => format!("assertion failed: {}", other.render()),
            }
        }
        // Not a structured assertion failure — fall back to generic rendering
        _ => format!("runtime returned err({})", reason.render()),
    }
}

fn compile_suite(path: &Path, source: &str) -> Result<TestSuite, TestRunnerError> {
    let filename = Some(path.display().to_string());

    // Inject stdlib modules (e.g. Assert) referenced by the test source.
    let mut enriched_source = source.to_string();
    inject_optional_stdlib(&mut enriched_source)
        .map_err(|e| TestRunnerError::Failure(format!("stdlib injection failed: {e}")))?;
    let source = &enriched_source;

    let tokens = scan_tokens(source).map_err(|error| TestRunnerError::SourceDiagnostic {
        message: error.to_string(),
        filename: filename.clone(),
        source: source.to_string(),
        offset: None,
    })?;

    let ast = parse_ast(&tokens).map_err(|error| TestRunnerError::SourceDiagnostic {
        message: error.to_string(),
        filename: filename.clone(),
        source: source.to_string(),
        offset: error.offset(),
    })?;

    let mut tests = ast
        .modules
        .iter()
        .flat_map(|module| {
            module
                .functions
                .iter()
                .filter(|function| {
                    !function.is_private()
                        && function.params.is_empty()
                        && function.name.starts_with("test_")
                })
                .map(|function| format!("{}.{}", module.name, function.name))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    tests.sort();

    resolve_ast(&ast).map_err(|error| TestRunnerError::SourceDiagnostic {
        message: error.to_string(),
        filename: filename.clone(),
        source: source.to_string(),
        offset: error.offset(),
    })?;

    infer_types(&ast).map_err(|error| TestRunnerError::SourceDiagnostic {
        message: error.to_string(),
        filename: filename.clone(),
        source: source.to_string(),
        offset: error.offset(),
    })?;

    let ir = lower_ast_to_ir(&ast).map_err(|error| TestRunnerError::Failure(error.to_string()))?;

    Ok(TestSuite { tests, ir })
}
