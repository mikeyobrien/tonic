use super::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

// ---------------------------------------------------------------------------
// packages.toml types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct PackagesManifest {
    #[serde(default)]
    packages: BTreeMap<String, PackageEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PackageEntry {
    source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    symlink: Option<bool>,
    #[serde(rename = "ref", skip_serializing_if = "Option::is_none")]
    git_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    commit: Option<String>,
    bins: Vec<String>,
    installed_at: String,
}

// ---------------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------------

fn tonic_home() -> PathBuf {
    if let Some(val) = std::env::var_os("TONIC_HOME") {
        return PathBuf::from(val);
    }
    dirs_or_home().join(".tonic")
}

fn dirs_or_home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn bin_dir() -> PathBuf {
    tonic_home().join("bin")
}

fn packages_dir() -> PathBuf {
    tonic_home().join("packages")
}

fn packages_toml_path() -> PathBuf {
    tonic_home().join("packages.toml")
}

// ---------------------------------------------------------------------------
// packages.toml I/O
// ---------------------------------------------------------------------------

fn load_packages_manifest() -> PackagesManifest {
    let path = packages_toml_path();
    match std::fs::read_to_string(&path) {
        Ok(contents) => toml::from_str(&contents).unwrap_or_default(),
        Err(_) => PackagesManifest::default(),
    }
}

fn save_packages_manifest(manifest: &PackagesManifest) -> Result<(), String> {
    let contents =
        toml::to_string_pretty(manifest).map_err(|e| format!("failed to serialize packages.toml: {e}"))?;
    std::fs::write(packages_toml_path(), contents)
        .map_err(|e| format!("failed to write packages.toml: {e}"))
}

// ---------------------------------------------------------------------------
// handle_install
// ---------------------------------------------------------------------------

pub(super) fn handle_install(args: Vec<String>) -> i32 {
    let mut source_arg: Option<String> = None;
    let mut copy = false;
    let mut force = false;
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" | "help" => {
                print_install_help();
                return EXIT_OK;
            }
            "--copy" => copy = true,
            "--force" => force = true,
            other if !other.starts_with('-') => {
                if source_arg.is_some() {
                    return CliDiagnostic::usage_with_hint(
                        "unexpected extra argument",
                        "run `tonic install --help` for usage",
                    )
                    .emit();
                }
                source_arg = Some(other.to_string());
            }
            other => {
                return CliDiagnostic::usage_with_hint(
                    format!("unknown flag '{other}'"),
                    "run `tonic install --help` for usage",
                )
                .emit();
            }
        }
    }

    let source = match source_arg {
        Some(s) => s,
        None => {
            return CliDiagnostic::usage_with_hint(
                "missing install source",
                "run `tonic install --help` for usage",
            )
            .emit();
        }
    };

    // Determine source type
    if source.starts_with("http://")
        || source.starts_with("https://")
        || source.starts_with("git://")
        || source.ends_with(".git")
    {
        return install_git(&source, force);
    }

    // Check if it looks like a bare name (registry)
    let path = Path::new(&source);
    if !path.exists() && !source.contains('/') && !source.starts_with('.') {
        return CliDiagnostic::failure_with_hint(
            format!("registry install not yet supported; use a path or git URL"),
            format!("try: tonic install ./{source}  (for a local path)"),
        );
    }

    install_local_path(&source, copy, force)
}

fn install_local_path(source: &str, copy: bool, force: bool) -> i32 {
    // Canonicalize
    let abs_path = match std::fs::canonicalize(source) {
        Ok(p) => p,
        Err(e) => {
            return CliDiagnostic::failure(format!("cannot resolve path '{source}': {e}")).emit();
        }
    };

    // Verify tonic.toml exists
    if !abs_path.join("tonic.toml").exists() {
        return CliDiagnostic::failure_with_hint(
            format!(
                "path does not appear to be a tonic project (no tonic.toml found)"
            ),
            format!("expected tonic.toml at {}", abs_path.join("tonic.toml").display()),
        );
    }

    // Read package name
    let (pkg_name, explicit_name) = read_package_name(&abs_path);

    // Check if ~/.tonic/bin/ exists yet (for PATH instructions later)
    let first_install = !bin_dir().exists();

    // Create directories
    if let Err(e) = std::fs::create_dir_all(bin_dir()) {
        return CliDiagnostic::failure(format!("failed to create {}: {e}", bin_dir().display())).emit();
    }
    if let Err(e) = std::fs::create_dir_all(packages_dir()) {
        return CliDiagnostic::failure(format!(
            "failed to create {}: {e}",
            packages_dir().display()
        ))
        .emit();
    }

    // Load manifest to check for conflicts
    let mut manifest = load_packages_manifest();

    // Discover binaries before linking (from the source path)
    let bins = match discover_binaries(&abs_path, &pkg_name, explicit_name) {
        Ok(bins) => bins,
        Err(msg) => return CliDiagnostic::failure(msg).emit(),
    };

    // Check binary name conflicts
    let is_reinstall = manifest.packages.contains_key(&pkg_name);
    if !force {
        for bin_name in &bins {
            for (other_pkg, entry) in &manifest.packages {
                if other_pkg == &pkg_name {
                    continue; // Reinstall of same package is fine
                }
                if entry.bins.contains(bin_name) {
                    return CliDiagnostic::failure_with_hint(
                        format!("binary '{bin_name}' already installed by package '{other_pkg}'"),
                        "use --force to overwrite",
                    );
                }
            }
        }
    }

    // Symlink or copy into ~/.tonic/packages/<name>/
    let pkg_dest = packages_dir().join(&pkg_name);
    if pkg_dest.exists() || pkg_dest.read_link().is_ok() {
        // Remove existing (reinstall)
        if pkg_dest.read_link().is_ok() {
            let _ = std::fs::remove_file(&pkg_dest);
        } else {
            let _ = std::fs::remove_dir_all(&pkg_dest);
        }
    }

    if copy {
        if let Err(e) = copy_dir_recursive(&abs_path, &pkg_dest) {
            return CliDiagnostic::failure(format!("failed to copy source: {e}")).emit();
        }
    } else {
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            if let Err(e) = symlink(&abs_path, &pkg_dest) {
                return CliDiagnostic::failure(format!("failed to create symlink: {e}")).emit();
            }
        }
        #[cfg(not(unix))]
        {
            return CliDiagnostic::failure("symlink install requires Unix; use --copy instead").emit();
        }
    }

    // Generate shims
    if let Err(msg) = generate_shims(&pkg_name, &pkg_dest, &bins) {
        return CliDiagnostic::failure(msg).emit();
    }

    // Update manifest
    let now = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    manifest.packages.insert(
        pkg_name.clone(),
        PackageEntry {
            source: "path".to_string(),
            path: Some(abs_path.display().to_string()),
            url: None,
            symlink: Some(!copy),
            git_ref: None,
            commit: None,
            bins: bins.clone(),
            installed_at: now,
        },
    );

    if let Err(msg) = save_packages_manifest(&manifest) {
        return CliDiagnostic::failure(msg).emit();
    }

    // Print summary
    let action = if is_reinstall { "Updated" } else { "Installed" };
    println!("{action} package '{pkg_name}'");
    println!(
        "  source: {} ({})",
        abs_path.display(),
        if copy { "copy" } else { "symlink" }
    );
    if bins.is_empty() {
        println!("  binaries: (none)");
    } else {
        println!("  binaries: {}", bins.join(", "));
    }

    if first_install {
        print_path_instructions();
    }

    EXIT_OK
}

fn install_git(_url: &str, _force: bool) -> i32 {
    // Git install will be implemented in Slice 3
    CliDiagnostic::failure("git URL install is not yet implemented; use a local path for now").emit()
}

// ---------------------------------------------------------------------------
// Binary discovery
// ---------------------------------------------------------------------------

fn discover_binaries(project_path: &Path, pkg_name: &str, explicit_name: bool) -> Result<Vec<String>, String> {
    let bin_dir = project_path.join("bin");

    if bin_dir.is_dir() {
        let mut bins = Vec::new();
        let entries = std::fs::read_dir(&bin_dir).map_err(|e| {
            format!("failed to read bin/ directory at {}: {e}", bin_dir.display())
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| format!("failed to read bin/ entry: {e}"))?;
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    // Skip hidden files
                    if !name.starts_with('.') {
                        bins.push(name.to_string());
                    }
                }
            }
        }
        bins.sort();
        if bins.is_empty() {
            return Err(format!(
                "no installable binaries found in {}/bin/; add executable files to bin/",
                project_path.display()
            ));
        }
        Ok(bins)
    } else if explicit_name {
        // Fallback: create a single shim named after the package, but only
        // when [package] name is explicitly set in tonic.toml.
        Ok(vec![pkg_name.to_string()])
    } else {
        Err(format!(
            "no installable binaries found; add executables to bin/ or set [package] name in tonic.toml",
        ))
    }
}

// ---------------------------------------------------------------------------
// Shim generation
// ---------------------------------------------------------------------------

fn generate_shims(pkg_name: &str, pkg_path: &Path, bins: &[String]) -> Result<(), String> {
    let bin_base = bin_dir();
    let pkg_bin_dir = pkg_path.join("bin");
    let has_bin_dir = pkg_bin_dir.is_dir();

    for bin_name in bins {
        let shim_path = bin_base.join(bin_name);

        let shim_content = if has_bin_dir && pkg_bin_dir.join(bin_name).exists() {
            // Delegate to the bin/ script
            let target = pkg_path.join("bin").join(bin_name);
            format!(
                "#!/bin/sh\n\
                 # Generated by tonic install — do not edit\n\
                 # Package: {pkg_name}, bin: {bin_name}\n\
                 set -eu\n\
                 exec '{}' \"$@\"\n",
                target.display()
            )
        } else {
            // Fallback: run via tonic run
            format!(
                "#!/bin/sh\n\
                 # Generated by tonic install — do not edit\n\
                 # Package: {pkg_name}, bin: {bin_name}\n\
                 set -eu\n\
                 exec tonic run '{}' \"$@\"\n",
                pkg_path.display()
            )
        };

        std::fs::write(&shim_path, &shim_content)
            .map_err(|e| format!("failed to write shim {}: {e}", shim_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o755);
            std::fs::set_permissions(&shim_path, perms)
                .map_err(|e| format!("failed to set permissions on {}: {e}", shim_path.display()))?;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Package name resolution
// ---------------------------------------------------------------------------

/// Returns `(name, explicit)` where `explicit` is true when the name came
/// from `[package] name` in `tonic.toml`, false when falling back to the
/// directory name.
fn read_package_name(project_path: &Path) -> (String, bool) {
    let manifest_path = project_path.join("tonic.toml");
    if let Ok(contents) = std::fs::read_to_string(&manifest_path) {
        if let Ok(value) = toml::from_str::<toml::Value>(&contents) {
            if let Some(name) = value
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
            {
                if !name.trim().is_empty() {
                    return (name.trim().to_string(), true);
                }
            }
        }
    }
    // Fallback: directory name
    let name = project_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();
    (name, false)
}

// ---------------------------------------------------------------------------
// Directory copy helper
// ---------------------------------------------------------------------------

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| format!("mkdir {}: {e}", dst.display()))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("read_dir {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| format!("read entry: {e}"))?;
        let ty = entry
            .file_type()
            .map_err(|e| format!("file_type: {e}"))?;
        let dest_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            std::fs::copy(entry.path(), &dest_path)
                .map_err(|e| format!("copy {}: {e}", entry.path().display()))?;
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// handle_uninstall
// ---------------------------------------------------------------------------

pub(super) fn handle_uninstall(args: Vec<String>) -> i32 {
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        print_uninstall_help();
        return EXIT_OK;
    }

    let name = match args.first() {
        Some(n) => n.clone(),
        None => {
            return CliDiagnostic::usage_with_hint(
                "missing package name",
                "run `tonic uninstall --help` for usage",
            )
            .emit();
        }
    };

    let mut manifest = load_packages_manifest();
    let entry = match manifest.packages.remove(&name) {
        Some(e) => e,
        None => {
            return CliDiagnostic::failure(format!("package '{name}' is not installed")).emit();
        }
    };

    // Remove shims
    for bin_name in &entry.bins {
        let shim_path = bin_dir().join(bin_name);
        if shim_path.exists() {
            let _ = std::fs::remove_file(&shim_path);
        }
    }

    // Remove cached package
    let pkg_path = packages_dir().join(&name);
    if pkg_path.read_link().is_ok() {
        let _ = std::fs::remove_file(&pkg_path);
    } else if pkg_path.exists() {
        let _ = std::fs::remove_dir_all(&pkg_path);
    }

    // Save updated manifest
    if let Err(msg) = save_packages_manifest(&manifest) {
        return CliDiagnostic::failure(msg).emit();
    }

    println!("Uninstalled package '{name}'");
    if !entry.bins.is_empty() {
        println!("  removed binaries: {}", entry.bins.join(", "));
    }

    EXIT_OK
}

// ---------------------------------------------------------------------------
// handle_installed
// ---------------------------------------------------------------------------

pub(super) fn handle_installed(args: Vec<String>) -> i32 {
    if matches!(
        args.first().map(String::as_str),
        Some("-h" | "--help" | "help")
    ) {
        print_installed_help();
        return EXIT_OK;
    }

    let manifest = load_packages_manifest();

    if manifest.packages.is_empty() {
        println!("No packages installed.");
        return EXIT_OK;
    }

    println!(
        "{:<20} {:<10} {:<30} {}",
        "PACKAGE", "SOURCE", "BINARIES", "INSTALLED"
    );
    println!("{}", "-".repeat(80));

    for (name, entry) in &manifest.packages {
        let source_display = match entry.source.as_str() {
            "path" => {
                if entry.symlink.unwrap_or(true) {
                    "path (link)"
                } else {
                    "path (copy)"
                }
            }
            "git" => "git",
            other => other,
        };

        let bins_display = if entry.bins.is_empty() {
            "(none)".to_string()
        } else {
            entry.bins.join(", ")
        };

        // Trim timestamp to date
        let date = entry
            .installed_at
            .split('T')
            .next()
            .unwrap_or(&entry.installed_at);

        println!("{:<20} {:<10} {:<30} {}", name, source_display, bins_display, date);
    }

    EXIT_OK
}

// ---------------------------------------------------------------------------
// CliDiagnostic extension for hint pattern
// ---------------------------------------------------------------------------

trait CliDiagnosticExt {
    fn failure_with_hint(message: impl Into<String>, hint: impl Into<String>) -> i32;
}

impl CliDiagnosticExt for CliDiagnostic {
    fn failure_with_hint(message: impl Into<String>, hint: impl Into<String>) -> i32 {
        let message = message.into();
        let hint = hint.into();
        eprintln!("error: {message}");
        eprintln!("{hint}");
        EXIT_FAILURE
    }
}

// ---------------------------------------------------------------------------
// Help text
// ---------------------------------------------------------------------------

fn print_install_help() {
    println!(
        "tonic install - Install a tonic module globally\n\n\
         Usage:\n  tonic install <source> [--copy] [--force]\n\n\
         Sources:\n  \
         <path>        Install from a local directory (default: symlink)\n  \
         <git-url>     Clone from a git repository\n  \
         <name>        Install from registry (not yet supported)\n\n\
         Options:\n  \
         --copy     Copy source instead of symlinking (local paths only)\n  \
         --force    Overwrite shims owned by another package\n\n\
         Examples:\n  \
         tonic install .                         # Install current project\n  \
         tonic install ../my-tool                # Install from relative path\n  \
         tonic install ../my-tool --copy         # Snapshot instead of symlink\n"
    );
}

fn print_uninstall_help() {
    println!(
        "tonic uninstall - Remove an installed tonic module\n\n\
         Usage:\n  tonic uninstall <name>\n\n\
         Removes shims from ~/.tonic/bin/, cached source from ~/.tonic/packages/,\n\
         and the entry from ~/.tonic/packages.toml.\n\n\
         Examples:\n  tonic uninstall my-tool\n"
    );
}

fn print_installed_help() {
    println!(
        "tonic installed - List installed tonic modules\n\n\
         Usage:\n  tonic installed\n\n\
         Shows all globally installed packages with their source, binaries,\n\
         and installation date.\n"
    );
}

fn print_path_instructions() {
    println!();
    println!("To use installed commands, add ~/.tonic/bin to your PATH:");
    println!();
    println!("  # bash/zsh");
    println!("  export PATH=\"$HOME/.tonic/bin:$PATH\"");
    println!();
    println!("  # fish");
    println!("  fish_add_path ~/.tonic/bin");
}

// ---------------------------------------------------------------------------
// Placeholder for run_placeholder compatibility
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_diag::EXIT_USAGE;

    #[test]
    fn install_help_exits_ok() {
        assert_eq!(handle_install(vec!["--help".to_string()]), EXIT_OK);
    }

    #[test]
    fn uninstall_help_exits_ok() {
        assert_eq!(handle_uninstall(vec!["--help".to_string()]), EXIT_OK);
    }

    #[test]
    fn installed_help_exits_ok() {
        assert_eq!(handle_installed(vec!["--help".to_string()]), EXIT_OK);
    }

    #[test]
    fn install_missing_source_exits_usage() {
        assert_eq!(handle_install(vec![]), EXIT_USAGE);
    }

    #[test]
    fn uninstall_missing_name_exits_usage() {
        assert_eq!(handle_uninstall(vec![]), EXIT_USAGE);
    }

    #[test]
    fn install_unknown_flag_exits_usage() {
        assert_eq!(handle_install(vec!["--bogus".to_string()]), EXIT_USAGE);
    }

    #[test]
    fn read_package_name_fallback_to_dir_name() {
        // A non-existent path should fall back to the directory name
        let (name, explicit) = read_package_name(Path::new("/tmp/nonexistent-test-pkg"));
        assert_eq!(name, "nonexistent-test-pkg");
        assert!(!explicit);
    }

    #[test]
    fn discover_binaries_errors_without_explicit_name_and_no_bin_dir() {
        // Project with tonic.toml but no [package] name and no bin/ dir
        // should error, not silently create a fallback shim.
        let dir = std::env::temp_dir().join("tonic-test-no-bins");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("tonic.toml"), "# empty manifest\n").unwrap();

        let result = discover_binaries(&dir, "tonic-test-no-bins", false);
        assert!(result.is_err(), "expected error when no bin/ and no explicit name");
        let msg = result.unwrap_err();
        assert!(msg.contains("no installable binaries found"), "got: {msg}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn packages_manifest_round_trip() {
        let mut manifest = PackagesManifest::default();
        manifest.packages.insert(
            "test-pkg".to_string(),
            PackageEntry {
                source: "path".to_string(),
                path: Some("/tmp/test".to_string()),
                url: None,
                symlink: Some(true),
                git_ref: None,
                commit: None,
                bins: vec!["test-bin".to_string()],
                installed_at: "2026-03-27T12:00:00Z".to_string(),
            },
        );

        let serialized = toml::to_string_pretty(&manifest).unwrap();
        let deserialized: PackagesManifest = toml::from_str(&serialized).unwrap();
        assert_eq!(deserialized.packages.len(), 1);
        assert!(deserialized.packages.contains_key("test-pkg"));
        assert_eq!(
            deserialized.packages["test-pkg"].bins,
            vec!["test-bin".to_string()]
        );
    }
}
