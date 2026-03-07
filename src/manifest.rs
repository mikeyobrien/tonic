use crate::deps::Lockfile;
use crate::lexer::scan_tokens;
use crate::parser::{parse_ast, Ast, Expr};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

// TODO: Add Registries struct when registry support is implemented.
// Registry URL will be configurable via [registries.default] in tonic.toml.

pub(crate) const STDLIB_SOURCES: &[(&str, &str)] = &[
    ("System", OPTIONAL_STDLIB_SYSTEM_SOURCE),
    ("String", OPTIONAL_STDLIB_STRING_SOURCE),
    ("Path", OPTIONAL_STDLIB_PATH_SOURCE),
];

const OPTIONAL_STDLIB_STRING_SOURCE: &str =
    "defmodule String do\n  def split(str, delimiter) do\n    host_call(:str_split, str, delimiter)\n  end\n\n  def replace(str, pattern, replacement) do\n    host_call(:str_replace, str, pattern, replacement)\n  end\n\n  def trim(str) do\n    host_call(:str_trim, str)\n  end\n\n  def trim_leading(str) do\n    host_call(:str_trim_leading, str)\n  end\n\n  def trim_trailing(str) do\n    host_call(:str_trim_trailing, str)\n  end\n\n  def starts_with(str, prefix) do\n    host_call(:str_starts_with, str, prefix)\n  end\n\n  def ends_with(str, suffix) do\n    host_call(:str_ends_with, str, suffix)\n  end\n\n  def contains(str, substr) do\n    host_call(:str_contains, str, substr)\n  end\n\n  def upcase(str) do\n    host_call(:str_upcase, str)\n  end\n\n  def downcase(str) do\n    host_call(:str_downcase, str)\n  end\n\n  def length(str) do\n    host_call(:str_length, str)\n  end\n\n  def to_charlist(str) do\n    host_call(:str_to_charlist, str)\n  end\n\n  def at(str, index) do\n    host_call(:str_at, str, index)\n  end\n\n  def slice(str, start, len) do\n    host_call(:str_slice, str, start, len)\n  end\n\n  def to_integer(str) do\n    host_call(:str_to_integer, str)\n  end\n\n  def to_float(str) do\n    host_call(:str_to_float, str)\n  end\n\n  def pad_leading(str, count, padding) do\n    host_call(:str_pad_leading, str, count, padding)\n  end\n\n  def pad_trailing(str, count, padding) do\n    host_call(:str_pad_trailing, str, count, padding)\n  end\n\n  def reverse(str) do\n    host_call(:str_reverse, str)\n  end\nend\n";

const OPTIONAL_STDLIB_PATH_SOURCE: &str =
    "defmodule Path do\n  def join(a, b) do\n    host_call(:path_join, a, b)\n  end\n\n  def dirname(path) do\n    host_call(:path_dirname, path)\n  end\n\n  def basename(path) do\n    host_call(:path_basename, path)\n  end\n\n  def extname(path) do\n    host_call(:path_extname, path)\n  end\n\n  def expand(path) do\n    host_call(:path_expand, path)\n  end\n\n  def relative_to(path, base) do\n    host_call(:path_relative_to, path, base)\n  end\nend\n";

const OPTIONAL_STDLIB_SYSTEM_SOURCE: &str =
    "defmodule System do\n  def run(command) do\n    host_call(:sys_run, command)\n  end\n\n  def sleep_ms(delay_ms) do\n    host_call(:sys_sleep_ms, delay_ms)\n  end\n\n  def retry_plan(status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after) do\n    host_call(:sys_retry_plan, status_code, attempt, max_attempts, base_delay_ms, max_delay_ms, jitter_ms, retry_after)\n  end\n\n  def log(level, event, fields) do\n    host_call(:sys_log, level, event, fields)\n  end\n\n  def path_exists(path) do\n    host_call(:sys_path_exists, path)\n  end\n\n  def list_files_recursive(path) do\n    host_call(:sys_list_files_recursive, path)\n  end\n\n  def ensure_dir(path) do\n    host_call(:sys_ensure_dir, path)\n  end\n\n  def remove_tree(path) do\n    host_call(:sys_remove_tree, path)\n  end\n\n  def write_text(path, content) do\n    host_call(:sys_write_text, path, content)\n  end\n\n  def append_text(path, content) do\n    host_call(:sys_append_text, path, content)\n  end\n\n  def write_text_atomic(path, content) do\n    host_call(:sys_write_text_atomic, path, content)\n  end\n\n  def lock_acquire(path) do\n    host_call(:sys_lock_acquire, path)\n  end\n\n  def lock_release(path) do\n    host_call(:sys_lock_release, path)\n  end\n\n  def read_text(path) do\n    host_call(:sys_read_text, path)\n  end\n\n  def read_stdin() do\n    host_call(:sys_read_stdin)\n  end\n\n  def http_request(method, url, headers, body, opts) do\n    host_call(:sys_http_request, method, url, headers, body, opts)\n  end\n\n  def env(name) do\n    host_call(:sys_env, name)\n  end\n\n  def which(name) do\n    host_call(:sys_which, name)\n  end\n\n  def cwd() do\n    host_call(:sys_cwd)\n  end\n\n  def argv() do\n    host_call(:sys_argv)\n  end\n\n  def random_token(bytes) do\n    host_call(:sys_random_token, bytes)\n  end\n\n  def hmac_sha256_hex(secret, message) do\n    host_call(:sys_hmac_sha256_hex, secret, message)\n  end\n\n  def constant_time_eq(left, right) do\n    host_call(:sys_constant_time_eq, left, right)\n  end\n\n  def discord_ed25519_verify(public_key_hex, signature_hex, timestamp, body) do\n    host_call(:sys_discord_ed25519_verify, public_key_hex, signature_hex, timestamp, body)\n  end\n\n  def http_listen(host, port) do\n    host_call(:sys_http_listen, host, port)\n  end\n\n  def http_accept(listener_id, timeout_ms) do\n    host_call(:sys_http_accept, listener_id, timeout_ms)\n  end\n\n  def http_read_request(connection_id) do\n    host_call(:sys_http_read_request, connection_id)\n  end\n\n  def http_write_response(connection_id, status, headers, body) do\n    host_call(:sys_http_write_response, connection_id, status, headers, body)\n  end\nend\n";

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
    let analysis = analyze_project_source(&source)?;

    if should_trace_module_loads() {
        for module_name in &analysis.module_names {
            trace_module_load("project", module_name);
        }
    }

    let stdlib_modules: &[(&str, &str)] = &[
        ("System", OPTIONAL_STDLIB_SYSTEM_SOURCE),
        ("String", OPTIONAL_STDLIB_STRING_SOURCE),
        ("Path", OPTIONAL_STDLIB_PATH_SOURCE),
    ];

    for (module_name, module_source) in stdlib_modules {
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

const STDLIB_MODULE_NAMES: &[&str] = &["System", "String", "Path"];

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
        Expr::Variable { .. } | Expr::Atom { .. } => false,
    }
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

pub(crate) fn load_project_manifest(project_root: &Path) -> Result<ProjectManifest, String> {
    let manifest_path = project_root.join("tonic.toml");
    let source = std::fs::read_to_string(&manifest_path).map_err(|_| {
        format!(
            "missing project manifest 'tonic.toml' at {}",
            manifest_path.display()
        )
    })?;

    parse_manifest(&source, project_root)
}

fn parse_manifest(source: &str, project_root: &Path) -> Result<ProjectManifest, String> {
    // Parse into Value to handle inline tables correctly
    let value: toml::Value =
        toml::from_str(source).map_err(|error| format!("invalid tonic.toml: {}", error))?;

    let entry = value.get("project").and_then(|p| p.get("entry"));

    let entry_str = match entry {
        None => return Err("invalid tonic.toml: missing required key project.entry".to_string()),
        Some(toml::Value::String(s)) => {
            if s.trim().is_empty() {
                return Err("invalid tonic.toml: project.entry cannot be empty".to_string());
            }
            s.trim().to_string()
        }
        Some(_) => return Err("invalid tonic.toml: project.entry must be a string".to_string()),
    };

    let dependencies = value
        .get("dependencies")
        .map(|deps| parse_dependencies_from_value(deps, project_root))
        .unwrap_or_else(|| Ok(Dependencies::default()))?;

    let package = value
        .get("package")
        .map(parse_package_metadata)
        .transpose()?;

    Ok(ProjectManifest {
        entry: PathBuf::from(entry_str),
        dependencies,
        package,
    })
}

fn parse_package_metadata(value: &toml::Value) -> Result<PackageMetadata, String> {
    let table = match value {
        toml::Value::Table(t) => t,
        _ => return Err("invalid tonic.toml: [package] must be a table".to_string()),
    };

    let name = extract_optional_string(table, "name", "package.name")?;
    let version = extract_optional_string(table, "version", "package.version")?;
    let description = extract_optional_string(table, "description", "package.description")?;
    let license = extract_optional_string(table, "license", "package.license")?;
    let repository = extract_optional_string(table, "repository", "package.repository")?;

    let authors = extract_string_array(table, "authors", "package.authors")?;
    let keywords = extract_string_array(table, "keywords", "package.keywords")?;

    Ok(PackageMetadata {
        name,
        version,
        description,
        license,
        authors,
        repository,
        keywords,
    })
}

fn extract_optional_string(
    table: &toml::value::Table,
    key: &str,
    display_path: &str,
) -> Result<Option<String>, String> {
    match table.get(key) {
        None => Ok(None),
        Some(toml::Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(format!(
            "invalid tonic.toml: {display_path} must be a string"
        )),
    }
}

fn extract_string_array(
    table: &toml::value::Table,
    key: &str,
    display_path: &str,
) -> Result<Vec<String>, String> {
    match table.get(key) {
        None => Ok(Vec::new()),
        Some(toml::Value::Array(arr)) => arr
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                toml::Value::String(s) => Ok(s.clone()),
                _ => Err(format!(
                    "invalid tonic.toml: {display_path}[{i}] must be a string"
                )),
            })
            .collect(),
        Some(_) => Err(format!(
            "invalid tonic.toml: {display_path} must be an array"
        )),
    }
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
        match value {
            // Shorthand string: `name = "~> 1.0"` — registry dep
            toml::Value::String(version_str) => {
                deps.registry.insert(
                    name.clone(),
                    RegistryDep {
                        version: version_str.clone(),
                        registry: None,
                    },
                );
            }
            toml::Value::Table(table) => {
                parse_dep_table(name, table, project_root, &mut deps)?;
            }
            _ => {
                return Err(format!(
                    "invalid tonic.toml: dependency '{name}' must be a version string or a table"
                ));
            }
        }
    }

    Ok(deps)
}

fn parse_dep_table(
    name: &str,
    table: &toml::value::Table,
    project_root: &Path,
    deps: &mut Dependencies,
) -> Result<(), String> {
    let has_path = table.contains_key("path");
    let has_git = table.contains_key("git");
    let has_version = table.contains_key("version");

    if has_path && has_git {
        return Err(format!(
            "invalid tonic.toml: dependency '{name}' cannot declare both 'path' and 'git' sources"
        ));
    }

    if has_path {
        let path_str = match table.get("path") {
            Some(toml::Value::String(s)) => s.clone(),
            _ => {
                return Err(format!(
                    "invalid tonic.toml: path dependency '{name}' has non-string 'path' value"
                ))
            }
        };

        let path = Path::new(&path_str);
        let resolved = if path.is_absolute() {
            path.to_path_buf()
        } else {
            project_root.join(path)
        };

        if !resolved.exists() {
            return Err(format!(
                "invalid tonic.toml: path dependency '{name}' points to non-existent path: {path_str}"
            ));
        }
        deps.path.insert(name.to_string(), resolved);
        return Ok(());
    }

    if has_git {
        let url = match table.get("git") {
            Some(toml::Value::String(s)) => s.clone(),
            _ => {
                return Err(format!(
                    "invalid tonic.toml: git dependency '{name}' has non-string 'git' value"
                ))
            }
        };
        let rev = match table.get("rev") {
            Some(toml::Value::String(s)) => s.clone(),
            Some(_) => {
                return Err(format!(
                    "invalid tonic.toml: git dependency '{name}' has non-string 'rev' value"
                ))
            }
            None => {
                return Err(format!(
                    "invalid tonic.toml: git dependency '{name}' missing 'rev'"
                ))
            }
        };
        deps.git.insert(name.to_string(), GitDep { url, rev });
        return Ok(());
    }

    if has_version {
        let version = match table.get("version") {
            Some(toml::Value::String(s)) => s.clone(),
            _ => {
                return Err(format!(
                "invalid tonic.toml: registry dependency '{name}' has non-string 'version' value"
            ))
            }
        };
        let registry = match table.get("registry") {
            None => None,
            Some(toml::Value::String(s)) => Some(s.clone()),
            Some(_) => {
                return Err(format!(
                "invalid tonic.toml: registry dependency '{name}' has non-string 'registry' value"
            ))
            }
        };
        deps.registry
            .insert(name.to_string(), RegistryDep { version, registry });
        return Ok(());
    }

    Err(format!(
        "invalid tonic.toml: dependency '{name}' must specify either a string 'path' or both string 'git' and 'rev'"
    ))
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
                let is_hidden = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| name.starts_with('.'));

                let is_target = path.file_name().is_some_and(|name| name == "target");

                if !is_hidden && !is_target {
                    pending_directories.push(path);
                }
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
                package: None,
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

    // --- [package] metadata tests ---

    #[test]
    fn parse_manifest_reads_full_package_metadata() {
        use super::PackageMetadata;

        let source = "[project]\nentry = \"main.tn\"\n\n\
            [package]\n\
            name = \"my_lib\"\n\
            version = \"0.2.0\"\n\
            description = \"A sample library\"\n\
            license = \"MIT\"\n\
            repository = \"https://github.com/example/my_lib\"\n\
            authors = [\"Alice\", \"Bob\"]\n\
            keywords = [\"tonic\", \"library\"]\n";

        let manifest = parse_manifest(source, Path::new("."))
            .expect("manifest with full package section should parse");

        assert_eq!(
            manifest.package,
            Some(PackageMetadata {
                name: Some("my_lib".to_string()),
                version: Some("0.2.0".to_string()),
                description: Some("A sample library".to_string()),
                license: Some("MIT".to_string()),
                authors: vec!["Alice".to_string(), "Bob".to_string()],
                repository: Some("https://github.com/example/my_lib".to_string()),
                keywords: vec!["tonic".to_string(), "library".to_string()],
            })
        );
    }

    #[test]
    fn parse_manifest_package_section_is_optional() {
        let source = "[project]\nentry = \"main.tn\"\n";
        let manifest = parse_manifest(source, Path::new("."))
            .expect("manifest without [package] should parse");
        assert_eq!(manifest.package, None);
    }

    #[test]
    fn parse_manifest_partial_package_metadata_is_valid() {
        let source = "[project]\nentry = \"main.tn\"\n\n[package]\nname = \"core\"\n";
        let manifest = parse_manifest(source, Path::new("."))
            .expect("manifest with partial [package] should parse");
        let pkg = manifest
            .package
            .expect("[package] section should be present");
        assert_eq!(pkg.name, Some("core".to_string()));
        assert_eq!(pkg.version, None);
        assert_eq!(pkg.description, None);
        assert_eq!(pkg.authors, Vec::<String>::new());
    }

    // --- Registry dependency tests ---

    #[test]
    fn parse_manifest_reads_registry_dep_shorthand() {
        use super::RegistryDep;

        let source = "[project]\nentry = \"main.tn\"\n\n[dependencies]\njson = \"~> 1.0\"\n";
        let manifest = parse_manifest(source, Path::new("."))
            .expect("manifest with shorthand registry dep should parse");

        assert_eq!(
            manifest.dependencies.registry.get("json"),
            Some(&RegistryDep {
                version: "~> 1.0".to_string(),
                registry: None,
            })
        );
    }

    #[test]
    fn parse_manifest_reads_registry_dep_table_form() {
        use super::RegistryDep;

        let source = "[project]\nentry = \"main.tn\"\n\n\
            [dependencies]\n\
            json = { version = \"^2.0\", registry = \"https://registry.example.com\" }\n";
        let manifest = parse_manifest(source, Path::new("."))
            .expect("manifest with table-form registry dep should parse");

        assert_eq!(
            manifest.dependencies.registry.get("json"),
            Some(&RegistryDep {
                version: "^2.0".to_string(),
                registry: Some("https://registry.example.com".to_string()),
            })
        );
    }

    #[test]
    fn parse_manifest_reads_registry_dep_table_form_without_registry_override() {
        use super::RegistryDep;

        let source =
            "[project]\nentry = \"main.tn\"\n\n[dependencies]\nhttp = { version = \"~> 0.5\" }\n";
        let manifest = parse_manifest(source, Path::new("."))
            .expect("manifest with table-form registry dep (no registry override) should parse");

        assert_eq!(
            manifest.dependencies.registry.get("http"),
            Some(&RegistryDep {
                version: "~> 0.5".to_string(),
                registry: None,
            })
        );
    }

    #[test]
    fn parse_manifest_rejects_package_name_non_string() {
        let source = "[project]\nentry = \"main.tn\"\n\n[package]\nname = 42\n";
        assert_eq!(
            parse_manifest(source, Path::new(".")),
            Err("invalid tonic.toml: package.name must be a string".to_string())
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
