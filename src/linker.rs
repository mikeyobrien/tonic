use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LinkerError {
    pub(crate) stage: LinkerStage,
    pub(crate) message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum LinkerStage {
    ToolNotFound,
    Compile,
}

impl LinkerError {
    fn tool_not_found(tool: &str) -> Self {
        Self {
            stage: LinkerStage::ToolNotFound,
            message: format!(
                "native toolchain not found: '{tool}' not found in PATH; \
                install gcc or clang to enable native compilation"
            ),
        }
    }

    fn compile_failed(tool: &str, stderr: String) -> Self {
        Self {
            stage: LinkerStage::Compile,
            message: format!("compile/link failed (tool: {tool}): {stderr}"),
        }
    }
}

impl std::fmt::Display for LinkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for LinkerError {}

/// Compile a C source file to a native executable.
///
/// Searches for an available C compiler in PATH order:
/// `clang`, `gcc`, `cc`
///
/// On success the executable is at `exe_path` with the executable bit set.
pub(crate) fn compile_c_to_executable(c_path: &Path, exe_path: &Path) -> Result<(), LinkerError> {
    let tool = find_c_compiler().ok_or_else(|| LinkerError::tool_not_found("cc"))?;

    let output = Command::new(&tool)
        .arg("-O2")
        .arg("-o")
        .arg(exe_path)
        .arg(c_path)
        .output()
        .map_err(|error| {
            LinkerError::tool_not_found(&tool);
            LinkerError {
                stage: LinkerStage::ToolNotFound,
                message: format!(
                    "failed to execute '{tool}': {error}; \
                    install gcc or clang to enable native compilation"
                ),
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(LinkerError::compile_failed(&tool, stderr));
    }

    // Ensure executable permission (compiler usually sets it, but be explicit).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let meta = std::fs::metadata(exe_path).map_err(|error| LinkerError {
            stage: LinkerStage::Compile,
            message: format!(
                "failed to stat output executable {}: {error}",
                exe_path.display()
            ),
        })?;
        let mut perms = meta.permissions();
        let mode = perms.mode();
        perms.set_mode(mode | 0o111);
        std::fs::set_permissions(exe_path, perms).map_err(|error| LinkerError {
            stage: LinkerStage::Compile,
            message: format!(
                "failed to set executable permissions on {}: {error}",
                exe_path.display()
            ),
        })?;
    }

    Ok(())
}

/// Detect the name of an available C compiler.
///
/// Tries candidates in order: `clang`, `gcc`, `cc`.
/// Returns the first one found in PATH.
pub(crate) fn find_c_compiler() -> Option<String> {
    for candidate in &["clang", "gcc", "cc"] {
        if which_exists(candidate) {
            return Some((*candidate).to_string());
        }
    }
    None
}

fn which_exists(program: &str) -> bool {
    // Use `which` if available; otherwise walk PATH manually.
    if let Ok(output) = Command::new("which").arg(program).output() {
        return output.status.success();
    }

    // Fallback: check PATH manually
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(program);
            if candidate.is_file() {
                return true;
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::find_c_compiler;

    #[test]
    fn find_c_compiler_returns_some_on_linux() {
        // On a typical Linux system at least `cc` or `gcc` is present.
        let result = find_c_compiler();
        assert!(
            result.is_some(),
            "expected at least one C compiler to be available"
        );
    }
}
