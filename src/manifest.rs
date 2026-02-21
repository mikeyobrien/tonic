use crate::lexer::scan_tokens;
use crate::parser::{parse_ast, Ast, Expr};
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectManifest {
    pub(crate) entry: PathBuf,
}

#[derive(Debug, Deserialize)]
struct RawManifest {
    project: Option<RawProject>,
}

#[derive(Debug, Deserialize)]
struct RawProject {
    entry: Option<String>,
}

const OPTIONAL_STDLIB_ENUM_SOURCE: &str =
    "defmodule Enum do\n  def identity() do\n    1\n  end\nend\n";

pub(crate) fn load_run_source(requested_path: &str) -> Result<String, String> {
    let path = Path::new(requested_path);

    if path.is_dir() {
        return load_run_source_from_project_root(path);
    }

    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read source file {requested_path}: {error}"))
}

fn load_run_source_from_project_root(project_root: &Path) -> Result<String, String> {
    let manifest = load_project_manifest(project_root)?;
    let entry_path = project_root.join(&manifest.entry);

    let mut project_sources = vec![read_source_file(&entry_path)?];

    for module_path in collect_project_module_paths(project_root, &entry_path)? {
        project_sources.push(read_source_file(&module_path)?);
    }

    let mut source = project_sources.join("\n\n");
    let analysis = analyze_project_source(&source);

    if should_trace_module_loads() {
        for module_name in &analysis.module_names {
            trace_module_load("project", module_name);
        }
    }

    if should_lazy_load_enum_stdlib(&analysis) {
        if !source.is_empty() {
            source.push_str("\n\n");
        }

        source.push_str(OPTIONAL_STDLIB_ENUM_SOURCE);

        if should_trace_module_loads() {
            trace_module_load("stdlib", "Enum");
        }
    }

    Ok(source)
}

#[derive(Debug, Default)]
struct ProjectSourceAnalysis {
    module_names: Vec<String>,
    references_enum: bool,
}

fn analyze_project_source(source: &str) -> ProjectSourceAnalysis {
    let Some(ast) = parse_project_ast(source) else {
        return ProjectSourceAnalysis {
            module_names: Vec::new(),
            references_enum: source.contains("Enum."),
        };
    };

    ProjectSourceAnalysis {
        module_names: collect_module_names(&ast),
        references_enum: ast_references_module(&ast, "Enum"),
    }
}

fn parse_project_ast(source: &str) -> Option<Ast> {
    let tokens = scan_tokens(source).ok()?;
    parse_ast(&tokens).ok()
}

fn collect_module_names(ast: &Ast) -> Vec<String> {
    ast.modules
        .iter()
        .map(|module| module.name.clone())
        .collect()
}

fn ast_references_module(ast: &Ast, module_name: &str) -> bool {
    ast.modules.iter().any(|module| {
        module
            .functions
            .iter()
            .any(|function| expr_references_module(&function.body, module_name))
    })
}

fn expr_references_module(expr: &Expr, module_name: &str) -> bool {
    match expr {
        Expr::Int { .. } => false,
        Expr::Call { callee, args, .. } => {
            let calls_module = callee
                .split_once('.')
                .is_some_and(|(prefix, _)| prefix == module_name);

            calls_module
                || args
                    .iter()
                    .any(|arg| expr_references_module(arg, module_name))
        }
        Expr::Question { value, .. } => expr_references_module(value, module_name),
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            expr_references_module(left, module_name) || expr_references_module(right, module_name)
        }
        Expr::Case {
            subject, branches, ..
        } => {
            expr_references_module(subject, module_name)
                || branches
                    .iter()
                    .any(|branch| expr_references_module(branch.body(), module_name))
        }
    }
}

fn should_lazy_load_enum_stdlib(analysis: &ProjectSourceAnalysis) -> bool {
    analysis.references_enum && !analysis.module_names.iter().any(|module| module == "Enum")
}

fn should_trace_module_loads() -> bool {
    std::env::var_os("TONIC_DEBUG_MODULE_LOADS").is_some()
}

fn trace_module_load(scope: &str, module_name: &str) {
    eprintln!("module-load {scope}:{module_name}");
}

fn load_project_manifest(project_root: &Path) -> Result<ProjectManifest, String> {
    let manifest_path = project_root.join("tonic.toml");
    let source = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read tonic.toml: {error}"))?;

    parse_manifest(&source)
}

fn parse_manifest(source: &str) -> Result<ProjectManifest, String> {
    let manifest: RawManifest =
        toml::from_str(source).map_err(|error| format!("invalid tonic.toml: {error}"))?;

    let entry = manifest
        .project
        .and_then(|project| project.entry)
        .map(|entry| entry.trim().to_string())
        .filter(|entry| !entry.is_empty())
        .ok_or_else(|| "invalid tonic.toml: missing required key project.entry".to_string())?;

    Ok(ProjectManifest {
        entry: PathBuf::from(entry),
    })
}

fn read_source_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read source file {}: {error}", path.display()))
}

fn collect_project_module_paths(
    project_root: &Path,
    entry_path: &Path,
) -> Result<Vec<PathBuf>, String> {
    let module_root = entry_path.parent().unwrap_or(project_root);
    let mut module_paths = collect_tonic_source_paths(module_root)?;

    module_paths.retain(|module_path| module_path != entry_path);

    Ok(module_paths)
}

fn collect_tonic_source_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut source_paths = Vec::new();
    let mut pending_directories = vec![root.to_path_buf()];

    while let Some(directory) = pending_directories.pop() {
        let entries = std::fs::read_dir(&directory).map_err(|error| {
            format!(
                "failed to read source directory {}: {error}",
                directory.display()
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|error| {
                format!(
                    "failed to read source directory {}: {error}",
                    directory.display()
                )
            })?;

            let path = entry.path();

            if path.is_dir() {
                pending_directories.push(path);
                continue;
            }

            if path.extension().and_then(|extension| extension.to_str()) == Some("tn") {
                source_paths.push(path);
            }
        }
    }

    source_paths.sort();

    Ok(source_paths)
}

#[cfg(test)]
mod tests {
    use super::{load_run_source, parse_manifest, ProjectManifest};
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;
    use std::path::PathBuf;

    #[test]
    fn parse_manifest_requires_project_entry() {
        assert_eq!(
            parse_manifest("[project]\nname = \"demo\"\n"),
            Err("invalid tonic.toml: missing required key project.entry".to_string())
        );
    }

    #[test]
    fn parse_manifest_reads_project_entry() {
        assert_eq!(
            parse_manifest("[project]\nname = \"demo\"\nentry = \"main.tn\"\n"),
            Ok(ProjectManifest {
                entry: PathBuf::from("main.tn")
            })
        );
    }

    #[test]
    fn load_run_source_reads_manifest_entry_when_path_is_directory() {
        let fixture_root = unique_fixture_root("run-source-from-project-root");

        std::fs::create_dir_all(&fixture_root)
            .expect("fixture setup should create project root directory");
        std::fs::write(
            fixture_root.join("tonic.toml"),
            "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
        )
        .expect("fixture setup should write tonic.toml");
        std::fs::create_dir_all(fixture_root.join("src"))
            .expect("fixture setup should create src directory");
        std::fs::write(
            fixture_root.join("src/main.tn"),
            "defmodule Demo do\n  def run() do\n    1\n  end\nend\n",
        )
        .expect("fixture setup should write entry source");

        let source = load_run_source(
            fixture_root
                .to_str()
                .expect("fixture root should be valid utf-8"),
        )
        .expect("run source loading should succeed");

        assert_eq!(
            source,
            "defmodule Demo do\n  def run() do\n    1\n  end\nend\n"
        );
    }

    #[test]
    fn load_run_source_includes_sibling_project_modules() {
        let fixture_root = unique_fixture_root("run-source-multi-module-project");

        std::fs::create_dir_all(fixture_root.join("src"))
            .expect("fixture setup should create src directory");
        std::fs::write(
            fixture_root.join("tonic.toml"),
            "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n",
        )
        .expect("fixture setup should write tonic.toml");
        std::fs::write(
            fixture_root.join("src/main.tn"),
            "defmodule Demo do\n  def run() do\n    Math.helper()\n  end\nend\n",
        )
        .expect("fixture setup should write entry source");
        std::fs::write(
            fixture_root.join("src/math.tn"),
            "defmodule Math do\n  def helper() do\n    1\n  end\nend\n",
        )
        .expect("fixture setup should write sibling source");

        let source = load_run_source(
            fixture_root
                .to_str()
                .expect("fixture root should be valid utf-8"),
        )
        .expect("run source loading should succeed");

        let tokens = scan_tokens(&source).expect("loader source should tokenize");
        let ast = parse_ast(&tokens).expect("loader source should parse");

        assert_eq!(
            ast.modules
                .iter()
                .map(|module| module.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Demo", "Math"]
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
