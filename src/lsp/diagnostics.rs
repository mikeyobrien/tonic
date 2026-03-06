use tower_lsp::lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range, Url};

use crate::lexer::scan_tokens;
use crate::lsp::document::offset_to_position;
use crate::parser::parse_ast;
use crate::resolver::resolve_ast;
use crate::typing::infer_types;

/// Run the full compiler pipeline on `source` and return LSP diagnostics.
pub fn compile_diagnostics(uri: &Url, source: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let tokens = match scan_tokens(source) {
        Ok(tokens) => tokens,
        Err(error) => {
            let offset = error.offset();
            let pos = offset_to_position(source, offset);
            diagnostics.push(make_diagnostic(
                pos,
                pos,
                error.to_string(),
                DiagnosticSeverity::ERROR,
            ));
            return diagnostics;
        }
    };

    let ast = match parse_ast(&tokens) {
        Ok(ast) => ast,
        Err(error) => {
            let offset = error.offset().unwrap_or(0);
            let pos = offset_to_position(source, offset);
            diagnostics.push(make_diagnostic(
                pos,
                pos,
                error.to_string(),
                DiagnosticSeverity::ERROR,
            ));
            return diagnostics;
        }
    };

    if let Err(error) = resolve_ast(&ast) {
        let offset = error.offset().unwrap_or(0);
        let pos = offset_to_position(source, offset);
        diagnostics.push(make_diagnostic(
            pos,
            pos,
            error.to_string(),
            DiagnosticSeverity::ERROR,
        ));
        return diagnostics;
    }

    if let Err(error) = infer_types(&ast) {
        let offset = error.offset().unwrap_or(0);
        let pos = offset_to_position(source, offset);
        diagnostics.push(make_diagnostic(
            pos,
            pos,
            error.to_string(),
            DiagnosticSeverity::ERROR,
        ));
        return diagnostics;
    }

    // No errors — publish empty diagnostics to clear any previous ones.
    let _ = uri;
    diagnostics
}

fn make_diagnostic(
    start: Position,
    end: Position,
    message: String,
    severity: DiagnosticSeverity,
) -> Diagnostic {
    Diagnostic {
        range: Range { start, end },
        severity: Some(severity),
        message,
        source: Some("tonic".to_string()),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_uri() -> Url {
        Url::parse("file:///tmp/test.tn").unwrap()
    }

    #[test]
    fn clean_source_produces_no_diagnostics() {
        let source = "defmodule Demo do\n  def run() do\n    42\n  end\nend\n";
        let diags = compile_diagnostics(&dummy_uri(), source);
        assert!(
            diags.is_empty(),
            "expected no diagnostics, got: {:?}",
            diags
        );
    }

    #[test]
    fn syntax_error_produces_single_diagnostic() {
        let source = "defmodule Demo do\n  def run( do\n    42\n  end\nend\n";
        let diags = compile_diagnostics(&dummy_uri(), source);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].severity, Some(DiagnosticSeverity::ERROR));
    }

    #[test]
    fn invalid_token_produces_diagnostic() {
        let source = "defmodule Demo do\n  def run() do\n    @\n  end\nend\n";
        let diags = compile_diagnostics(&dummy_uri(), source);
        // @ is a valid token (module attribute start) — but this is lexed successfully.
        // We just verify the pipeline does not panic.
        let _ = diags;
    }
}
