use crate::ir::{lower_ast_to_ir, IrProgram};
use crate::lexer::scan_tokens;
use crate::parser::parse_ast;
use crate::resolver::resolve_ast;
use crate::runtime::{evaluate_named_function, RuntimeValue};
use crate::typing::infer_types;
use serde_json::json;
use std::path::{Path, PathBuf};

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
    pub results: Vec<TestCaseResult>,
}

impl TestRunReport {
    pub fn succeeded(&self) -> bool {
        self.failed == 0
    }

    pub fn render_text(&self) -> Vec<String> {
        let mut lines = Vec::new();

        for result in &self.results {
            match result.status {
                TestCaseStatus::Passed => {
                    lines.push(format!("test {} ... ok", result.id));
                }
                TestCaseStatus::Failed => {
                    lines.push(format!("test {} ... FAILED", result.id));
                    if let Some(error) = &result.error {
                        lines.push(format!("  error: {error}"));
                    }
                }
            }
        }

        let status = if self.succeeded() { "ok" } else { "FAILED" };
        lines.push(format!(
            "test result: {status}. {} passed; {} failed; {} total",
            self.passed, self.failed, self.total
        ));

        lines
    }

    pub fn render_json(&self) -> serde_json::Value {
        json!({
            "status": if self.succeeded() { "ok" } else { "failed" },
            "total": self.total,
            "passed": self.passed,
            "failed": self.failed,
            "results": self
                .results
                .iter()
                .map(|result| {
                    json!({
                        "id": result.id,
                        "status": match result.status {
                            TestCaseStatus::Passed => "passed",
                            TestCaseStatus::Failed => "failed",
                        },
                        "error": result.error,
                    })
                })
                .collect::<Vec<_>>(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TestRunnerError {
    Failure(String),
    SourceDiagnostic {
        message: String,
        source: String,
        offset: Option<usize>,
    },
}

struct TestSuite {
    tests: Vec<String>,
    ir: IrProgram,
}

pub fn run(path: &str) -> Result<TestRunReport, TestRunnerError> {
    let target = Path::new(path);
    let test_files = discover_test_files(target)?;
    let mut results = Vec::new();

    for file in test_files {
        let source = std::fs::read_to_string(&file).map_err(|error| {
            TestRunnerError::Failure(format!(
                "failed to read source file {}: {error}",
                file.display()
            ))
        })?;

        let suite = compile_suite(&file, &source)?;

        for test_name in suite.tests {
            match evaluate_named_function(&suite.ir, &test_name) {
                Ok(RuntimeValue::ResultErr(reason)) => {
                    results.push(TestCaseResult {
                        id: test_name,
                        status: TestCaseStatus::Failed,
                        error: Some(format!("runtime returned err({})", reason.render())),
                    });
                }
                Ok(_) => {
                    results.push(TestCaseResult {
                        id: test_name,
                        status: TestCaseStatus::Passed,
                        error: None,
                    });
                }
                Err(error) => {
                    results.push(TestCaseResult {
                        id: test_name,
                        status: TestCaseStatus::Failed,
                        error: Some(error.to_string()),
                    });
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
        results,
    })
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

fn compile_suite(path: &Path, source: &str) -> Result<TestSuite, TestRunnerError> {
    let tokens = scan_tokens(source).map_err(|error| TestRunnerError::SourceDiagnostic {
        message: format!("{}: {}", path.display(), error),
        source: source.to_string(),
        offset: None,
    })?;

    let ast = parse_ast(&tokens).map_err(|error| TestRunnerError::SourceDiagnostic {
        message: format!("{}: {}", path.display(), error),
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
        message: format!("{}: {}", path.display(), error),
        source: source.to_string(),
        offset: error.offset(),
    })?;

    infer_types(&ast).map_err(|error| TestRunnerError::SourceDiagnostic {
        message: format!("{}: {}", path.display(), error),
        source: source.to_string(),
        offset: error.offset(),
    })?;

    let ir = lower_ast_to_ir(&ast).map_err(|error| TestRunnerError::Failure(error.to_string()))?;

    Ok(TestSuite { tests, ir })
}
