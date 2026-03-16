use super::{Dependencies, GitDep, PackageMetadata, ProjectManifest, RegistryDep};
use std::path::{Path, PathBuf};

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

pub(super) fn parse_manifest(source: &str, project_root: &Path) -> Result<ProjectManifest, String> {
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

pub(super) fn parse_package_metadata(value: &toml::Value) -> Result<PackageMetadata, String> {
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

pub(super) fn extract_optional_string(
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

pub(super) fn extract_string_array(
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

pub(super) fn parse_dependencies_from_value(
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

pub(super) fn parse_dep_table(
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

pub(super) fn read_source_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path)
        .map_err(|error| format!("failed to read source file {}: {error}", path.display()))
}

pub(super) fn collect_project_module_paths(
    project_root: &Path,
    entry_path: &Path,
) -> Result<Vec<PathBuf>, String> {
    let module_root = entry_path.parent().unwrap_or(project_root);
    let mut module_paths = collect_tonic_source_paths(module_root)?;

    module_paths.retain(|module_path| module_path != entry_path);

    Ok(module_paths)
}

pub(super) fn collect_tonic_source_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
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
