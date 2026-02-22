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

pub(crate) fn format_source(source: &str) -> String {
    let normalized = normalize_newlines(source);
    let mut indent_level = 0usize;
    let mut branch_body_open = false;
    let mut formatted_lines = Vec::new();
    let mut previous_was_blank = false;

    for raw_line in normalized.lines() {
        let trimmed = raw_line.trim();

        if trimmed.is_empty() {
            if !previous_was_blank {
                formatted_lines.push(String::new());
                previous_was_blank = true;
            }
            continue;
        }

        previous_was_blank = false;

        if branch_body_open && (is_branch_line(trimmed) || is_block_boundary_line(trimmed)) {
            indent_level = indent_level.saturating_sub(1);
            branch_body_open = false;
        }

        if is_block_closing_line(trimmed) {
            indent_level = indent_level.saturating_sub(1);
        }

        let mut rendered = String::new();
        rendered.push_str(&"  ".repeat(indent_level));
        rendered.push_str(trimmed);
        formatted_lines.push(rendered);

        if is_block_reopen_line(trimmed) {
            indent_level += 1;
        }

        if opens_do_block(trimmed) {
            indent_level += 1;
        }

        if is_branch_line(trimmed) {
            indent_level += 1;
            branch_body_open = true;
        }
    }

    while formatted_lines.last().is_some_and(|line| line.is_empty()) {
        formatted_lines.pop();
    }

    if formatted_lines.is_empty() {
        return String::new();
    }

    let mut output = formatted_lines.join("\n");
    output.push('\n');
    output
}

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

fn normalize_newlines(source: &str) -> String {
    source.replace("\r\n", "\n").replace('\r', "\n")
}

fn is_block_boundary_line(line: &str) -> bool {
    is_block_closing_line(line) || is_block_reopen_line(line)
}

fn is_block_closing_line(line: &str) -> bool {
    starts_with_keyword(line, "end")
        || starts_with_keyword(line, "else")
        || starts_with_keyword(line, "rescue")
        || starts_with_keyword(line, "catch")
        || starts_with_keyword(line, "after")
}

fn is_block_reopen_line(line: &str) -> bool {
    starts_with_keyword(line, "else")
        || starts_with_keyword(line, "rescue")
        || starts_with_keyword(line, "catch")
        || starts_with_keyword(line, "after")
}

fn opens_do_block(line: &str) -> bool {
    line == "do" || line.ends_with(" do")
}

fn is_branch_line(line: &str) -> bool {
    line.contains("->")
}

fn starts_with_keyword(line: &str, keyword: &str) -> bool {
    if line == keyword {
        return true;
    }

    let Some(rest) = line.strip_prefix(keyword) else {
        return false;
    };

    rest.chars()
        .next()
        .is_some_and(|ch| ch.is_ascii_whitespace() || matches!(ch, ',' | ')' | ']' | '}' | ';'))
}

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
}
