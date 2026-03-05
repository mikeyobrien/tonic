use std::collections::HashMap;
use tower_lsp::lsp_types::Url;

/// In-memory store for open document contents.
#[derive(Debug, Default)]
pub struct DocumentStore {
    documents: HashMap<String, String>,
}

impl DocumentStore {
    pub fn open(&mut self, uri: &Url, text: String) {
        self.documents.insert(uri.to_string(), text);
    }

    pub fn update(&mut self, uri: &Url, text: String) {
        self.documents.insert(uri.to_string(), text);
    }

    pub fn close(&mut self, uri: &Url) {
        self.documents.remove(&uri.to_string());
    }

    pub fn get(&self, uri: &Url) -> Option<&str> {
        self.documents.get(&uri.to_string()).map(String::as_str)
    }
}

/// Convert a byte offset within `source` to an LSP (line, character) position.
/// LSP positions are 0-indexed.
pub fn offset_to_position(source: &str, offset: usize) -> tower_lsp::lsp_types::Position {
    let safe_offset = offset.min(source.len());
    let before = &source[..safe_offset];
    let line = before.bytes().filter(|&b| b == b'\n').count() as u32;
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let character = before[line_start..].chars().count() as u32;
    tower_lsp::lsp_types::Position { line, character }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn offset_to_position_first_line() {
        let source = "hello world";
        let pos = offset_to_position(source, 6);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 6);
    }

    #[test]
    fn offset_to_position_second_line() {
        let source = "line one\nline two";
        let pos = offset_to_position(source, 9);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn offset_to_position_clamps_to_source_length() {
        let source = "abc";
        let pos = offset_to_position(source, 999);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 3);
    }
}
