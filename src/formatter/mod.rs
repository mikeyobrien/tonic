mod engine;

use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FormatMode {
    Write,
    Check,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FormatReport {
    pub(crate) checked_files: usize,
    pub(crate) changed_files: usize,
}

pub(crate) fn format_path(path: &str, mode: FormatMode) -> Result<FormatReport, String> {
    let requested_path = Path::new(path);

    if !requested_path.exists() {
        return Err(format!("path does not exist: {}", requested_path.display()));
    }

    let files = collect_tonic_files(requested_path)?;
    if files.is_empty() {
        return Err(format!(
            "no .tn source files found at {}",
            requested_path.display()
        ));
    }

    let mut report = FormatReport {
        checked_files: 0,
        changed_files: 0,
    };

    for file in files {
        let source = fs::read_to_string(&file)
            .map_err(|error| format!("failed to read source file {}: {error}", file.display()))?;
        let formatted = format_source(&source);

        report.checked_files += 1;

        if source != formatted {
            report.changed_files += 1;

            if mode == FormatMode::Write {
                fs::write(&file, formatted).map_err(|error| {
                    format!("failed to write formatted file {}: {error}", file.display())
                })?;
            }
        }
    }

    Ok(report)
}

/// Format a Tonic source string.
///
/// Uses a token-driven two-pass approach: lexer tokens are segmented into
/// logical lines (Pass 1), then indentation is applied (Pass 2).
///
/// Known limitation: comments (`# ...`) are stripped by the lexer and
/// not preserved in the formatted output. This is a known alpha-stage
/// limitation; comment preservation requires a comment-aware token stream.
///
/// If lexing fails (malformed source), the original normalized source is
/// returned unchanged to avoid corrupting code with syntax errors.
pub(crate) fn format_source(source: &str) -> String {
    engine::format_source_inner(source)
}

// ---------------------------------------------------------------------------
// File collection
// ---------------------------------------------------------------------------

fn collect_tonic_files(path: &Path) -> Result<Vec<PathBuf>, String> {
    if path.is_file() {
        return Ok(vec![path.to_path_buf()]);
    }

    let mut files = Vec::new();
    collect_tonic_files_recursive(path, &mut files)?;
    files.sort();
    Ok(files)
}

fn collect_tonic_files_recursive(directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut entries = fs::read_dir(directory)
        .map_err(|error| format!("failed to read directory {}: {error}", directory.display()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| {
            format!(
                "failed to read directory entry in {}: {error}",
                directory.display()
            )
        })?;

    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();

        if path.is_dir() {
            collect_tonic_files_recursive(&path, files)?;
            continue;
        }

        if path
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("tn"))
        {
            files.push(path);
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::format_source;

    #[test]
    fn format_source_indents_nested_do_blocks() {
        let source = "defmodule Demo do\ndef run() do\nif true do\n1\nelse\n2\nend\nend\nend\n";

        assert_eq!(
            format_source(source),
            "defmodule Demo do\n  def run() do\n    if true do\n      1\n    else\n      2\n    end\n  end\nend\n"
        );
    }

    #[test]
    fn format_source_indents_case_branches() {
        let source =
            "defmodule Demo do\ndef run() do\ncase 2 do\n1 ->\n10\n2 ->\n20\nend\nend\nend\n";

        assert_eq!(
            format_source(source),
            "defmodule Demo do\n  def run() do\n    case 2 do\n      1 ->\n        10\n      2 ->\n        20\n    end\n  end\nend\n"
        );
    }

    #[test]
    fn format_source_collapses_extra_blank_lines() {
        let source = "defmodule Demo do\n\n\n  def run() do\n    1\n  end\nend\n\n";

        assert_eq!(
            format_source(source),
            "defmodule Demo do\n\n  def run() do\n    1\n  end\nend\n"
        );
    }

    #[test]
    fn format_source_is_idempotent_nested_if() {
        let already = "defmodule Demo do\n  def run() do\n    if true do\n      1\n    else\n      2\n    end\n  end\nend\n";
        let second = format_source(already);
        assert_eq!(
            already, second,
            "formatting already-formatted code must be idempotent"
        );
    }

    #[test]
    fn format_source_is_idempotent_case_branches() {
        let already = "defmodule Demo do\n  def run() do\n    case 2 do\n      1 ->\n        10\n      2 ->\n        20\n    end\n  end\nend\n";
        let second = format_source(already);
        assert_eq!(
            already, second,
            "formatting already-formatted code must be idempotent"
        );
    }

    #[test]
    fn format_source_struct_syntax_round_trip() {
        let source = "defmodule User do\ndefstruct name: \"\", age: 0\ndef run(user) do\ncase %User{user | age: 43} do\n%User{name: name} ->\n%User{name: name}\n_ ->\n%User{}\nend\nend\nend\n";
        let first = format_source(source);
        let second = format_source(&first);
        assert_eq!(first, second, "struct syntax format must be idempotent");
    }

    #[test]
    fn format_source_try_rescue_idempotent() {
        let source = "defmodule Demo do\ndef run() do\ntry do\nraise \"err\"\nrescue\n_ -> \"caught\"\nend\nend\nend\n";
        let result = format_source(source);
        let second = format_source(&result);
        assert_eq!(result, second, "try/rescue format must be idempotent");
    }

    #[test]
    fn format_source_function_clauses_idempotent() {
        let already = "defmodule Demo do\n  defp fib(0) do\n    0\n  end\n\n  defp fib(1) do\n    1\n  end\n\n  defp fib(n) when n > 1 do\n    fib(n - 1) + fib(n - 2)\n  end\nend\n";
        let second = format_source(already);
        assert_eq!(
            already, second,
            "function clauses with blank lines must be idempotent"
        );
    }
}
