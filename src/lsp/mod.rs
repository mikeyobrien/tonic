pub mod definition;
pub mod diagnostics;
pub mod document;
pub mod hover;

use std::sync::Arc;
use tokio::sync::Mutex;
use tower_lsp::jsonrpc::Result as LspResult;
use tower_lsp::lsp_types::{
    DidChangeTextDocumentParams, DidCloseTextDocumentParams, DidOpenTextDocumentParams,
    DidSaveTextDocumentParams, GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
    HoverProviderCapability, InitializeParams, InitializeResult, InitializedParams,
    MessageType, OneOf, ServerCapabilities, TextDocumentSyncCapability, TextDocumentSyncKind,
    Url,
};
use tower_lsp::{Client, LanguageServer, LspService, Server};

use document::DocumentStore;

/// LSP server for the Tonic language.
#[derive(Debug)]
struct TonicLspServer {
    client: Client,
    documents: Arc<Mutex<DocumentStore>>,
}

impl TonicLspServer {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(Mutex::new(DocumentStore::default())),
        }
    }

    async fn publish_diagnostics(&self, uri: Url, source: String) {
        let diags = diagnostics::compile_diagnostics(&uri, &source);
        self.client
            .publish_diagnostics(uri, diags, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for TonicLspServer {
    async fn initialize(&self, _params: InitializeParams) -> LspResult<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: None,
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "tonic language server initialized")
            .await;
    }

    async fn shutdown(&self) -> LspResult<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        {
            let mut store = self.documents.lock().await;
            store.open(&uri, text.clone());
        }
        self.publish_diagnostics(uri, text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            let mut store = self.documents.lock().await;
            store.update(&uri, change.text);
        }
        // Don't re-run diagnostics on every keystroke — wait for save.
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = {
            let store = self.documents.lock().await;
            store.get(&uri).map(str::to_owned)
        };
        if let Some(text) = source {
            self.publish_diagnostics(uri, text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut store = self.documents.lock().await;
        store.close(&params.text_document.uri);
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> LspResult<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let source = {
            let store = self.documents.lock().await;
            store.get(&uri).map(str::to_owned)
        };

        let result = source
            .as_deref()
            .and_then(|src| definition::find_definition(&uri, src, position));
        Ok(result)
    }

    async fn hover(&self, params: HoverParams) -> LspResult<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let source = {
            let store = self.documents.lock().await;
            store.get(&uri).map(str::to_owned)
        };

        let result = source
            .as_deref()
            .and_then(|src| hover::hover_info(src, position));
        Ok(result)
    }
}

/// Entry point for `tonic lsp` subcommand.  Runs the LSP server over stdio.
pub fn run_lsp_server() {
    let rt = tokio::runtime::Runtime::new().expect("failed to build tokio runtime");
    rt.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let (service, socket) = LspService::new(TonicLspServer::new);
        Server::new(stdin, stdout, socket).serve(service).await;
    });
}

#[cfg(test)]
mod tests {
    use super::document::offset_to_position;
    use tower_lsp::lsp_types::Url;

    fn dummy_uri() -> Url {
        Url::parse("file:///tmp/test.tn").unwrap()
    }

    #[test]
    fn diagnostics_pipeline_runs_without_panic() {
        let source = "defmodule Demo do\n  def run() do\n    42\n  end\nend\n";
        let diags = super::diagnostics::compile_diagnostics(&dummy_uri(), source);
        assert!(diags.is_empty());
    }

    #[test]
    fn diagnostics_pipeline_reports_parse_error() {
        let source = "defmodule Demo do\n  def run( do\n    42\n  end\nend\n";
        let diags = super::diagnostics::compile_diagnostics(&dummy_uri(), source);
        assert_eq!(diags.len(), 1);
    }

    #[test]
    fn offset_to_position_maps_zero_offset() {
        let source = "hello";
        let pos = offset_to_position(source, 0);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }
}
