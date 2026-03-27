use crate::ir::{lower_ast_to_ir, IrProgram};
use crate::lexer::scan_tokens;
use crate::manifest::inject_optional_stdlib;
use crate::parser::parse_ast;
use crate::resolver::resolve_ast;
use crate::runtime::{evaluate_named_function, RuntimeValue};
use crate::typing::infer_types;
use serde_json::json;
use std::collections::HashSet;
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
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestRunReport {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub skipped: usize,
    pub duration: Duration,
    pub results: Vec<TestCaseResult>,
}

impl TestRunReport {
    pub fn succeeded(&self) -> bool {
        self.failed == 0
    }

    pub fn render_text(&self) -> Vec<String> {
        let color = AnsiColor::from_env();
        let mut lines = Vec::new();

        for result in &self.results {
            let duration = format_duration(result.duration);
            match result.status {
                TestCaseStatus::Passed => {
                    lines.push(format!(
                        "test {} ... {}ok{} ({duration})",
                        result.id, color.green, color.reset
                    ));
                }
                TestCaseStatus::Failed => {
                    lines.push(format!(
                        "test {} ... {}FAILED{} ({duration})",
                        result.id, color.red, color.reset
                    ));
                    if let Some(error) = &result.error {
                        lines.push(format!("  error: {error}"));
                    }
                }
                TestCaseStatus::Skipped => {
                    let reason = result
                        .error
                        .as_deref()
                        .filter(|s| !s.is_empty())
                        .map(|s| format!(" ({s})"))
                        .unwrap_or_default();
                    lines.push(format!(
                        "test {} ... {}skipped{}{reason} ({duration})",
                        result.id, color.yellow, color.reset
                    ));
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
            lines.push(format!(
                "{}{}Failures:{}",
                color.bold, color.red, color.reset
            ));
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

        let (status, status_color) = if self.succeeded() {
            ("ok", color.green)
        } else {
            ("FAILED", color.red)
        };
        let total_duration = format_duration(self.duration);
        let skipped_part = if self.skipped > 0 {
            format!("; {} skipped", self.skipped)
        } else {
            String::new()
        };
        lines.push(format!(
            "test result: {status_color}{status}{reset}. {} passed; {} failed{skipped_part}; {} total; finished in {total_duration}",
            self.passed, self.failed, self.total,
            reset = color.reset
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
                    TestCaseStatus::Skipped => "skipped",
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
            "skipped": self.skipped,
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
    setup_modules: HashSet<String>,
    teardown_modules: HashSet<String>,
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
    seed: Option<u64>,
    timeout: Option<u64>,
) -> Result<TestRunReport, TestRunnerError> {
    let target = Path::new(path);
    let test_files = discover_test_files(target)?;

    // Collect all (test_name, ir) pairs, then optionally shuffle.
    let mut test_cases: Vec<(String, IrProgram)> = Vec::new();
    let mut setup_modules: HashSet<String> = HashSet::new();
    let mut teardown_modules: HashSet<String> = HashSet::new();

    for file in test_files {
        let source = std::fs::read_to_string(&file).map_err(|error| {
            TestRunnerError::Failure(format!(
                "failed to read source file {}: {error}",
                file.display()
            ))
        })?;

        let suite = compile_suite(&file, &source)?;
        setup_modules.extend(suite.setup_modules.into_iter());
        teardown_modules.extend(suite.teardown_modules.into_iter());

        for test_name in suite.tests {
            if let Some(pattern) = filter {
                if !test_name.contains(pattern) {
                    continue;
                }
            }
            test_cases.push((test_name, suite.ir.clone()));
        }
    }

    if let Some(seed) = seed {
        shuffle(&mut test_cases, seed);
    }

    let mut results = Vec::new();
    let run_start = Instant::now();

    let timeout_duration = timeout.map(Duration::from_millis);

    for (test_name, ir) in &test_cases {
        let test_start = Instant::now();

        // Run setup/0 if the test's module has one.
        let module_name = test_name.split('.').next().unwrap_or("");
        if setup_modules.contains(module_name) {
            let setup_fn = format!("{module_name}.setup");
            match run_with_timeout(ir, &setup_fn, timeout_duration) {
                TestExecResult::Ok(RuntimeValue::ResultErr(reason)) => {
                    let error = format!("setup failed: {}", format_assertion_failure(&reason));
                    results.push(TestCaseResult {
                        id: test_name.clone(),
                        status: TestCaseStatus::Failed,
                        error: Some(error),
                        duration: test_start.elapsed(),
                    });
                    if fail_fast {
                        break;
                    }
                    continue;
                }
                TestExecResult::Err(error) => {
                    let error = format!("setup failed: {error}");
                    results.push(TestCaseResult {
                        id: test_name.clone(),
                        status: TestCaseStatus::Failed,
                        error: Some(error),
                        duration: test_start.elapsed(),
                    });
                    if fail_fast {
                        break;
                    }
                    continue;
                }
                TestExecResult::TimedOut(ms) => {
                    let error = format!("setup timed out after {ms}ms");
                    results.push(TestCaseResult {
                        id: test_name.clone(),
                        status: TestCaseStatus::Failed,
                        error: Some(error),
                        duration: test_start.elapsed(),
                    });
                    if fail_fast {
                        break;
                    }
                    continue;
                }
                TestExecResult::Ok(_) => {} // setup succeeded, proceed to test
            }
        }

        let (mut status, mut error) = match run_with_timeout(ir, test_name, timeout_duration) {
            TestExecResult::Ok(RuntimeValue::ResultErr(reason)) => {
                if is_test_skipped(&reason) {
                    let skip_reason = extract_skip_reason(&reason);
                    (TestCaseStatus::Skipped, Some(skip_reason))
                } else {
                    let err = format_assertion_failure(&reason);
                    (TestCaseStatus::Failed, Some(err))
                }
            }
            TestExecResult::Ok(_) => (TestCaseStatus::Passed, None),
            TestExecResult::Err(e) => (TestCaseStatus::Failed, Some(e.to_string())),
            TestExecResult::TimedOut(ms) => (
                TestCaseStatus::Failed,
                Some(format!("test timed out after {ms}ms")),
            ),
        };

        // Run teardown/0 if the test's module has one (always, regardless of test outcome).
        if teardown_modules.contains(module_name) {
            let teardown_fn = format!("{module_name}.teardown");
            let teardown_err = match run_with_timeout(ir, &teardown_fn, timeout_duration) {
                TestExecResult::Ok(RuntimeValue::ResultErr(reason)) => Some(format!(
                    "teardown failed: {}",
                    format_assertion_failure(&reason)
                )),
                TestExecResult::Err(e) => Some(format!("teardown failed: {e}")),
                TestExecResult::TimedOut(ms) => Some(format!("teardown timed out after {ms}ms")),
                TestExecResult::Ok(_) => None,
            };
            if let Some(td_err) = teardown_err {
                match status {
                    TestCaseStatus::Passed => {
                        status = TestCaseStatus::Failed;
                        error = Some(td_err);
                    }
                    TestCaseStatus::Failed => {
                        let existing = error.unwrap_or_default();
                        error = Some(format!("{existing}\n({td_err})"));
                    }
                    TestCaseStatus::Skipped => {
                        // Teardown failure overrides skip — the test has a real problem
                        status = TestCaseStatus::Failed;
                        error = Some(td_err);
                    }
                }
            }
        }

        let is_failure = status == TestCaseStatus::Failed;
        results.push(TestCaseResult {
            id: test_name.clone(),
            status,
            error,
            duration: test_start.elapsed(),
        });
        if is_failure && fail_fast {
            break;
        }
    }

    let passed = results
        .iter()
        .filter(|result| result.status == TestCaseStatus::Passed)
        .count();
    let skipped = results
        .iter()
        .filter(|result| result.status == TestCaseStatus::Skipped)
        .count();
    let failed = results.len().saturating_sub(passed + skipped);

    Ok(TestRunReport {
        total: results.len(),
        passed,
        failed,
        skipped,
        duration: run_start.elapsed(),
        results,
    })
}

/// Result of running a test function, possibly with a timeout.
enum TestExecResult {
    Ok(RuntimeValue),
    Err(String),
    TimedOut(u64),
}

/// Run a named function with an optional timeout. When timeout is None, runs directly.
fn run_with_timeout(ir: &IrProgram, fn_name: &str, timeout: Option<Duration>) -> TestExecResult {
    match timeout {
        None => match evaluate_named_function(ir, fn_name) {
            Ok(val) => TestExecResult::Ok(val),
            Err(e) => TestExecResult::Err(e.to_string()),
        },
        Some(limit) => {
            let ir = ir.clone();
            let fn_name = fn_name.to_string();
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let result = evaluate_named_function(&ir, &fn_name);
                let _ = tx.send(result);
            });
            match rx.recv_timeout(limit) {
                Ok(Ok(val)) => TestExecResult::Ok(val),
                Ok(Err(e)) => TestExecResult::Err(e.to_string()),
                Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                    TestExecResult::TimedOut(limit.as_millis() as u64)
                }
                Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                    TestExecResult::Err("test thread panicked".to_string())
                }
            }
        }
    }
}

/// Splitmix64 PRNG — simple, fast, dependency-free.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e3779b97f4a7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z ^ (z >> 31)
}

/// Fisher-Yates shuffle using splitmix64 PRNG.
fn shuffle<T>(slice: &mut [T], seed: u64) {
    let mut state = seed;
    for i in (1..slice.len()).rev() {
        let j = (splitmix64(&mut state) as usize) % (i + 1);
        slice.swap(i, j);
    }
}

struct AnsiColor {
    green: &'static str,
    red: &'static str,
    yellow: &'static str,
    bold: &'static str,
    reset: &'static str,
}

impl AnsiColor {
    fn from_env() -> Self {
        if std::env::var("NO_COLOR").is_ok() {
            Self {
                green: "",
                red: "",
                yellow: "",
                bold: "",
                reset: "",
            }
        } else {
            Self {
                green: "\x1b[32m",
                red: "\x1b[31m",
                yellow: "\x1b[33m",
                bold: "\x1b[1m",
                reset: "\x1b[0m",
            }
        }
    }
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

/// Check if a ResultErr value represents a test skip: `{:test_skipped, reason}`.
fn is_test_skipped(reason: &RuntimeValue) -> bool {
    matches!(reason,
        RuntimeValue::Tuple(tag, _) if matches!(tag.as_ref(), RuntimeValue::Atom(a) if a == "test_skipped")
    )
}

/// Extract the skip reason string from a `{:test_skipped, reason}` tuple.
fn extract_skip_reason(reason: &RuntimeValue) -> String {
    if let RuntimeValue::Tuple(_, reason_val) = reason {
        if let RuntimeValue::String(s) = reason_val.as_ref() {
            return s.clone();
        }
    }
    String::new()
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
                    let mut expected = None;
                    let mut actual = None;
                    let mut missing_keys = None;
                    let mut mismatched_keys = None;
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
                                            "assert_match" => "assert_match",
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
                                RuntimeValue::Atom(k) if k == "expected" => {
                                    expected = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "actual" => {
                                    actual = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "missing_keys" => {
                                    missing_keys = Some(val.render());
                                }
                                RuntimeValue::Atom(k) if k == "mismatched_keys" => {
                                    mismatched_keys = Some(val.render());
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
                        "assert_match" => {
                            if let Some(e) = expected {
                                lines.push(format!("  expected: {e}"));
                            }
                            if let Some(a) = actual {
                                lines.push(format!("  actual:   {a}"));
                            }
                            if let Some(m) = missing_keys {
                                lines.push(format!("  missing keys:    {m}"));
                            }
                            if let Some(m) = mismatched_keys {
                                lines.push(format!("  mismatched keys: {m}"));
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

    let mut tests = Vec::new();
    let mut setup_modules = HashSet::new();
    let mut teardown_modules = HashSet::new();

    for module in &ast.modules {
        let has_setup = module.functions.iter().any(|function| {
            !function.is_private() && function.params.is_empty() && function.name == "setup"
        });
        if has_setup {
            setup_modules.insert(module.name.clone());
        }

        let has_teardown = module.functions.iter().any(|function| {
            !function.is_private() && function.params.is_empty() && function.name == "teardown"
        });
        if has_teardown {
            teardown_modules.insert(module.name.clone());
        }

        for function in &module.functions {
            if !function.is_private()
                && function.params.is_empty()
                && function.name.starts_with("test_")
            {
                tests.push(format!("{}.{}", module.name, function.name));
            }
        }
    }
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

    Ok(TestSuite {
        tests,
        ir,
        setup_modules,
        teardown_modules,
    })
}
