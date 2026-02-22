use crate::deps::Lockfile;
use crate::lexer::scan_tokens;
use crate::parser::{parse_ast, Ast, Expr};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectManifest {
    pub(crate) entry: PathBuf,
    pub(crate) dependencies: Dependencies,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Dependencies {
    pub(crate) path: HashMap<String, PathBuf>,
    pub(crate) git: HashMap<String, GitDep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDep {
    pub(crate) url: String,
    pub(crate) rev: String,
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

    // FIX: Load dependency sources into the runtime
    let dependency_sources = load_dependency_sources(project_root, &manifest.dependencies)?;
    project_sources.extend(dependency_sources);

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

/// Load source files from all dependencies (path and git)
fn load_dependency_sources(
    project_root: &Path,
    manifest_dependencies: &Dependencies,
) -> Result<Vec<String>, String> {
    let mut dependency_sources = Vec::new();

    let lockfile = match Lockfile::load(project_root)? {
        Some(lockfile) => lockfile,
        None if !manifest_dependencies.path.is_empty() || !manifest_dependencies.git.is_empty() => {
            return Err(
                "dependencies declared in tonic.toml but tonic.lock is missing; run `tonic deps lock` or `tonic deps sync`"
                    .to_string(),
            );
        }
        None => return Ok(dependency_sources),
    };

    let deps_dir = Lockfile::deps_dir(project_root);

    // Load path dependencies
    for (name, path_dep) in &lockfile.path_deps {
        let dep_path = Path::new(&path_dep.path);
        if !dep_path.exists() {
            return Err(format!(
                "locked path dependency '{}' not found at {}; run `tonic deps lock`",
                name, path_dep.path
            ));
        }

        for source_path in collect_tonic_source_paths(dep_path)? {
            if should_trace_module_loads() {
                trace_module_load("dep:path", &source_path.to_string_lossy());
            }
            dependency_sources.push(read_source_file(&source_path)?);
        }
    }

    // Load git dependencies from cache
    for name in lockfile.git_deps.keys() {
        let dep_path = deps_dir.join(name);
        if !dep_path.exists() {
            return Err(format!(
                "cached git dependency '{}' not found at {}; run `tonic deps sync`",
                name,
                dep_path.display()
            ));
        }

        for source_path in collect_tonic_source_paths(&dep_path)? {
            if should_trace_module_loads() {
                trace_module_load("dep:git", &source_path.to_string_lossy());
            }
            dependency_sources.push(read_source_file(&source_path)?);
        }
    }

    Ok(dependency_sources)
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
        module.functions.iter().any(|function| {
            function
                .guard()
                .is_some_and(|guard| expr_references_module(guard, module_name))
                || expr_references_module(&function.body, module_name)
        })
    })
}

fn expr_references_module(expr: &Expr, module_name: &str) -> bool {
    match expr {
        Expr::Int { .. } | Expr::Bool { .. } | Expr::Nil { .. } | Expr::String { .. } => false,
        Expr::Tuple { items, .. } | Expr::List { items, .. } => items
            .iter()
            .any(|item| expr_references_module(item, module_name)),
        Expr::Map { entries, .. } | Expr::Keyword { entries, .. } => entries
            .iter()
            .any(|entry| expr_references_module(&entry.value, module_name)),
        Expr::Call { callee, args, .. } => {
            let calls_module = callee
                .split_once('.')
                .is_some_and(|(prefix, _)| prefix == module_name);

            calls_module
                || args
                    .iter()
                    .any(|arg| expr_references_module(arg, module_name))
        }
        Expr::Fn { body, .. } => expr_references_module(body, module_name),
        Expr::Invoke { callee, args, .. } => {
            expr_references_module(callee, module_name)
                || args
                    .iter()
                    .any(|arg| expr_references_module(arg, module_name))
        }
        Expr::Question { value, .. } | Expr::Unary { value, .. } => {
            expr_references_module(value, module_name)
        }
        Expr::Binary { left, right, .. } | Expr::Pipe { left, right, .. } => {
            expr_references_module(left, module_name) || expr_references_module(right, module_name)
        }
        Expr::Case {
            subject, branches, ..
        } => {
            expr_references_module(subject, module_name)
                || branches.iter().any(|branch| {
                    branch
                        .guard()
                        .is_some_and(|guard| expr_references_module(guard, module_name))
                        || expr_references_module(branch.body(), module_name)
                })
        }
        Expr::Group { inner, .. } => expr_references_module(inner, module_name),
        Expr::Variable { .. } | Expr::Atom { .. } => false,
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

pub(crate) fn load_project_manifest(project_root: &Path) -> Result<ProjectManifest, String> {
    let manifest_path = project_root.join("tonic.toml");
    let source = std::fs::read_to_string(&manifest_path)
        .map_err(|error| format!("failed to read tonic.toml: {error}"))?;

    parse_manifest(&source, project_root)
}

fn parse_manifest(source: &str, project_root: &Path) -> Result<ProjectManifest, String> {
    // Parse into Value to handle inline tables correctly
    let value: toml::Value =
        toml::from_str(source).map_err(|error| format!("invalid tonic.toml: {}", error))?;

    let entry = value
        .get("project")
        .and_then(|p| p.get("entry"))
        .and_then(|e| e.as_str())
        .map(|e| e.trim().to_string())
        .filter(|e| !e.is_empty())
        .ok_or_else(|| "invalid tonic.toml: missing required key project.entry".to_string())?;

    let dependencies = value
        .get("dependencies")
        .map(|deps| parse_dependencies_from_value(deps, project_root))
        .unwrap_or_else(|| Ok(Dependencies::default()))?;

    Ok(ProjectManifest {
        entry: PathBuf::from(entry),
        dependencies,
    })
}

fn parse_dependencies_from_value(
    deps_value: &toml::Value,
    project_root: &Path,
) -> Result<Dependencies, String> {
    let mut deps = Dependencies::default();

    let deps_table = match deps_value {
        toml::Value::Table(t) => t,
        _ => return Ok(Dependencies::default()),
    };

    // Each key in the table is a dependency name
    for (name, value) in deps_table {
        let table = match value {
            toml::Value::Table(t) => t,
            _ => {
                return Err(format!(
                    "invalid tonic.toml: dependency '{}' must specify either a string 'path' or both string 'git' and 'rev'",
                    name
                ));
            }
        };

        let has_path = table.contains_key("path");
        let has_git = table.contains_key("git");

        if has_path && has_git {
            return Err(format!(
                "invalid tonic.toml: dependency '{}' cannot declare both 'path' and 'git' sources",
                name
            ));
        }

        if has_path {
            let Some(path_val) = table.get("path") else {
                return Err(format!(
                    "invalid tonic.toml: path dependency '{}' has non-string 'path' value",
                    name
                ));
            };
            let Some(path_str) = path_val.as_str() else {
                return Err(format!(
                    "invalid tonic.toml: path dependency '{}' has non-string 'path' value",
                    name
                ));
            };

            let path = Path::new(path_str);
            let resolved = if path.is_absolute() {
                path.to_path_buf()
            } else {
                project_root.join(path)
            };

            if !resolved.exists() {
                return Err(format!(
                    "invalid tonic.toml: path dependency '{}' points to non-existent path: {}",
                    name, path_str
                ));
            }
            deps.path.insert(name.clone(), resolved);
            continue;
        }

        if has_git {
            let Some(git_val) = table.get("git") else {
                return Err(format!(
                    "invalid tonic.toml: git dependency '{}' has non-string 'git' value",
                    name
                ));
            };
            let Some(url) = git_val.as_str() else {
                return Err(format!(
                    "invalid tonic.toml: git dependency '{}' has non-string 'git' value",
                    name
                ));
            };
            let Some(rev_val) = table.get("rev") else {
                return Err(format!(
                    "invalid tonic.toml: git dependency '{}' missing 'rev'",
                    name
                ));
            };
            let Some(rev) = rev_val.as_str() else {
                return Err(format!(
                    "invalid tonic.toml: git dependency '{}' has non-string 'rev' value",
                    name
                ));
            };
            deps.git.insert(
                name.clone(),
                GitDep {
                    url: url.to_string(),
                    rev: rev.to_string(),
                },
            );
            continue;
        }

        return Err(format!(
            "invalid tonic.toml: dependency '{}' must specify either a string 'path' or both string 'git' and 'rev'",
            name
        ));
    }

    Ok(deps)
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
    use super::{load_run_source, parse_manifest, Dependencies, ProjectManifest};
    use crate::lexer::scan_tokens;
    use crate::parser::parse_ast;
    use std::path::{Path, PathBuf};

    #[test]
    fn parse_manifest_requires_project_entry() {
        assert_eq!(
            parse_manifest("[project]\nname = \"demo\"\n", Path::new(".")),
            Err("invalid tonic.toml: missing required key project.entry".to_string())
        );
    }

    #[test]
    fn parse_manifest_reads_project_entry() {
        assert_eq!(
            parse_manifest(
                "[project]\nname = \"demo\"\nentry = \"main.tn\"\n",
                Path::new("."),
            ),
            Ok(ProjectManifest {
                entry: PathBuf::from("main.tn"),
                dependencies: Dependencies::default(),
            })
        );
    }

    #[test]
    fn parse_manifest_resolves_relative_path_dependencies_from_project_root() {
        let fixture_root = unique_fixture_root("manifest-relative-path-dependency");
        let project_root = fixture_root.join("app");
        let dep_root = fixture_root.join("shared_dep");

        std::fs::create_dir_all(&project_root)
            .expect("fixture setup should create project root directory");
        std::fs::create_dir_all(&dep_root)
            .expect("fixture setup should create dependency directory");

        let manifest = parse_manifest(
            "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nshared_dep = { path = \"../shared_dep\" }\n",
            &project_root,
        )
        .expect("manifest should parse with relative path dependency");

        let resolved = manifest
            .dependencies
            .path
            .get("shared_dep")
            .expect("dependency should be present")
            .canonicalize()
            .expect("resolved dependency path should canonicalize");
        let expected = dep_root
            .canonicalize()
            .expect("expected dependency path should canonicalize");

        assert_eq!(resolved, expected);
    }

    #[test]
    fn parse_manifest_rejects_dependency_without_path_or_git_source() {
        assert_eq!(
            parse_manifest(
                "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nbroken = { rev = \"abc123\" }\n",
                Path::new("."),
            ),
            Err(
                "invalid tonic.toml: dependency 'broken' must specify either a string 'path' or both string 'git' and 'rev'"
                    .to_string()
            )
        );
    }

    #[test]
    fn parse_manifest_rejects_path_dependency_with_non_string_path_value() {
        assert_eq!(
            parse_manifest(
                "[project]\nname = \"demo\"\nentry = \"src/main.tn\"\n\n[dependencies]\nbroken = { path = 42 }\n",
                Path::new("."),
            ),
            Err(
                "invalid tonic.toml: path dependency 'broken' has non-string 'path' value"
                    .to_string()
            )
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
