use crate::deps::Lockfile;
use crate::lexer::scan_tokens;
use crate::parser::{parse_ast, Ast, Expr};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[path = "manifest_stdlib.rs"]
mod stdlib;
pub(crate) use stdlib::STDLIB_SOURCES;

#[path = "manifest_parse.rs"]
mod parse;
pub(crate) use parse::load_project_manifest;
use parse::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectManifest {
    pub(crate) entry: PathBuf,
    pub(crate) dependencies: Dependencies,
    /// Optional package metadata for registry publishing.
    pub(crate) package: Option<PackageMetadata>,
}

/// Package metadata for registry publishing.
/// All fields are optional to preserve backward compatibility with
/// manifests that predate registry support.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct PackageMetadata {
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) license: Option<String>,
    pub(crate) authors: Vec<String>,
    pub(crate) repository: Option<String>,
    pub(crate) keywords: Vec<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Dependencies {
    pub(crate) path: HashMap<String, PathBuf>,
    pub(crate) git: HashMap<String, GitDep>,
    pub(crate) registry: HashMap<String, RegistryDep>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDep {
    pub(crate) url: String,
    pub(crate) rev: String,
}

/// A registry dependency resolved by semver version range.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegistryDep {
    /// Semver version requirement, e.g. "~> 1.0" or "^2.1".
    pub(crate) version: String,
    /// Override registry URL; uses `[registries.default]` when `None`.
    pub(crate) registry: Option<String>,
}

// Registry URL will be configurable via [registries.default] in tonic.toml
// when registry support is implemented.

pub(crate) fn load_run_source(requested_path: &str) -> Result<String, String> {
    let path = Path::new(requested_path);

    if path.is_dir() {
        return load_run_source_from_project_root(path);
    }

    let mut source = std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read source file {requested_path}: {error}"))?;

    inject_optional_stdlib(&mut source)?;

    Ok(source)
}

fn load_run_source_from_project_root(project_root: &Path) -> Result<String, String> {
    let manifest = load_project_manifest(project_root)?;
    let entry_path = project_root.join(&manifest.entry);

    if !entry_path.exists() {
        return Err(format!(
            "project entry path '{}' does not exist",
            manifest.entry.display()
        ));
    }
    if !entry_path.is_file() {
        return Err(format!(
            "project entry path '{}' is not a file",
            manifest.entry.display()
        ));
    }

    let mut project_sources = vec![read_source_file(&entry_path)?];

    for module_path in collect_project_module_paths(project_root, &entry_path)? {
        project_sources.push(read_source_file(&module_path)?);
    }

    // FIX: Load dependency sources into the runtime
    let dependency_sources = load_dependency_sources(project_root, &manifest.dependencies)?;
    project_sources.extend(dependency_sources);

    let mut source = project_sources.join("\n\n");

    if should_trace_module_loads() {
        let analysis = analyze_project_source(&source)?;
        for module_name in &analysis.module_names {
            trace_module_load("project", module_name);
        }
    }

    inject_optional_stdlib(&mut source)?;

    Ok(source)
}

/// Load source files from all dependencies (path and git)
fn load_dependency_sources(
    project_root: &Path,
    manifest_dependencies: &Dependencies,
) -> Result<Vec<String>, String> {
    // Registry deps are not yet resolvable — surface an actionable error.
    if let Some(name) = manifest_dependencies.registry.keys().next() {
        return Err(format!(
            "registry dependency '{}': registry dependencies are not yet supported; use git or path deps",
            name
        ));
    }

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
    referenced_modules: Vec<String>,
}

const STDLIB_MODULE_NAMES: &[&str] = &[
    "System", "String", "Path", "IO", "List", "Map", "Enum", "Integer", "Float", "Tuple",
    "Assert",
];

fn analyze_project_source(source: &str) -> Result<ProjectSourceAnalysis, String> {
    let Some(ast) = parse_project_ast(source) else {
        let mut referenced_modules = Vec::new();
        for module_name in STDLIB_MODULE_NAMES {
            if source.contains(&format!("{module_name}.")) {
                referenced_modules.push(module_name.to_string());
            }
        }

        return Ok(ProjectSourceAnalysis {
            module_names: Vec::new(),
            referenced_modules,
        });
    };

    let mut module_names = Vec::new();

    for module in &ast.modules {
        module_names.push(module.name.clone());
    }

    let mut referenced_modules = Vec::new();
    for module_name in STDLIB_MODULE_NAMES {
        if ast_references_module(&ast, module_name) {
            referenced_modules.push(module_name.to_string());
        }
    }

    Ok(ProjectSourceAnalysis {
        module_names,
        referenced_modules,
    })
}

fn parse_project_ast(source: &str) -> Option<Ast> {
    let tokens = scan_tokens(source).ok()?;
    parse_ast(&tokens).ok()
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
        Expr::Int { .. }
        | Expr::Float { .. }
        | Expr::Bool { .. }
        | Expr::Nil { .. }
        | Expr::String { .. } => false,
        Expr::InterpolatedString { segments, .. } => segments.iter().any(|segment| match segment {
            crate::parser::InterpolationSegment::Expr { expr } => {
                expr_references_module(expr, module_name)
            }
            _ => false,
        }),
        Expr::Tuple { items, .. } | Expr::List { items, .. } | Expr::Bitstring { items, .. } => {
            items
                .iter()
                .any(|item| expr_references_module(item, module_name))
        }
        Expr::Map { entries, .. } => entries.iter().any(|entry| {
            expr_references_module(entry.key(), module_name)
                || expr_references_module(entry.value(), module_name)
        }),
        Expr::Struct {
            module, entries, ..
        } => {
            module == module_name
                || entries
                    .iter()
                    .any(|entry| expr_references_module(&entry.value, module_name))
        }
        Expr::Keyword { entries, .. } => entries
            .iter()
            .any(|entry| expr_references_module(&entry.value, module_name)),
        Expr::MapUpdate { base, updates, .. } => {
            expr_references_module(base, module_name)
                || updates
                    .iter()
                    .any(|entry| expr_references_module(&entry.value, module_name))
        }
        Expr::StructUpdate {
            module,
            base,
            updates,
            ..
        } => {
            module == module_name
                || expr_references_module(base, module_name)
                || updates
                    .iter()
                    .any(|entry| expr_references_module(&entry.value, module_name))
        }
        Expr::FieldAccess { base, .. } => expr_references_module(base, module_name),
        Expr::IndexAccess { base, index, .. } => {
            expr_references_module(base, module_name) || expr_references_module(index, module_name)
        }
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
        Expr::For {
            generators,
            into,
            reduce,
            body,
            ..
        } => {
            generators.iter().any(|generator| {
                expr_references_module(generator.source(), module_name)
                    || generator
                        .guard()
                        .is_some_and(|guard| expr_references_module(guard, module_name))
            }) || into
                .as_ref()
                .is_some_and(|into_expr| expr_references_module(into_expr, module_name))
                || reduce
                    .as_ref()
                    .is_some_and(|reduce_expr| expr_references_module(reduce_expr, module_name))
                || expr_references_module(body, module_name)
        }
        Expr::Group { inner, .. } => expr_references_module(inner, module_name),
        Expr::Try { body, rescue, .. } => {
            expr_references_module(body, module_name)
                || rescue.iter().any(|branch| {
                    branch
                        .guard()
                        .is_some_and(|guard| expr_references_module(guard, module_name))
                        || expr_references_module(branch.body(), module_name)
                })
        }
        Expr::Raise { error, .. } => expr_references_module(error, module_name),
        Expr::Block { exprs, .. } => exprs.iter().any(|e| expr_references_module(e, module_name)),
        Expr::Variable { .. } | Expr::Atom { .. } => false,
    }
}

/// Analyze source for stdlib module references and inject any needed stdlib modules.
pub(crate) fn inject_optional_stdlib(source: &mut String) -> Result<(), String> {
    let analysis = analyze_project_source(source)?;

    for (module_name, module_source) in STDLIB_SOURCES {
        if should_lazy_load_optional_stdlib(&analysis, module_name) {
            if !source.is_empty() {
                source.push_str("\n\n");
            }

            source.push_str(module_source);

            if should_trace_module_loads() {
                trace_module_load("stdlib", module_name);
            }
        }
    }

    Ok(())
}

fn should_lazy_load_optional_stdlib(analysis: &ProjectSourceAnalysis, module_name: &str) -> bool {
    analysis
        .referenced_modules
        .iter()
        .any(|candidate| candidate == module_name)
        && !analysis
            .module_names
            .iter()
            .any(|defined_module| defined_module == module_name)
}

fn should_trace_module_loads() -> bool {
    std::env::var_os("TONIC_DEBUG_MODULE_LOADS").is_some()
}

fn trace_module_load(scope: &str, module_name: &str) {
    eprintln!("module-load {scope}:{module_name}");
}

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod tests;
