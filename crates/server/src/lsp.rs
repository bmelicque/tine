use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::utils::normalize_file_url;
use crate::{span_to_range, Backend};

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        // TODO:
        let _root = params.root_uri.and_then(|u| u.to_file_path().ok());

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                        SemanticTokensRegistrationOptions {
                            text_document_registration_options: TextDocumentRegistrationOptions {
                                document_selector: Some(vec![DocumentFilter {
                                    language: Some("tine".into()),
                                    scheme: None,
                                    pattern: None,
                                }]),
                            },
                            semantic_tokens_options: SemanticTokensOptions {
                                work_done_progress_options: Default::default(),
                                legend: self.semantic_legend.clone(),
                                range: None,
                                full: Some(SemanticTokensFullOptions::Bool(true)),
                            },
                            static_registration_options: Default::default(),
                        },
                    ),
                ),
                ..Default::default()
            },
            server_info: None,
        })
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = normalize_file_url(&params.text_document.uri).unwrap();
        self.open_files
            .write()
            .unwrap()
            .insert(uri.clone(), params.text_document.text);
        if let Ok(path) = uri.to_file_path() {
            self.run_project_analysis(path).await;
        } else {
            self.client
                .log_message(
                    MessageType::WARNING,
                    format!("didOpen: cannot convert uri {} to path", uri),
                )
                .await;
        }
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = normalize_file_url(&params.text_document.uri).unwrap();
        if let Some(file) = self.open_files.write().unwrap().get_mut(&uri) {
            *file = params.content_changes[0].text.clone();
        }
        if let Ok(path) = uri.to_file_path() {
            self.run_project_analysis(path).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = normalize_file_url(&params.text_document.uri).unwrap();
        self.open_files.write().unwrap().remove(&uri);
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = normalize_file_url(&params.text_document.uri).unwrap();
        let Some(module_id) = self.find_module(&uri) else {
            return Ok(None);
        };
        let src = &self.sources.read().unwrap()[&module_id];
        let data = self.tokens_to_semantic(module_id, src);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = &params.text_document_position_params.text_document.uri;
        let uri = normalize_file_url(uri).unwrap();
        let Some(module_id) = self.find_module(&uri) else {
            return Ok(None);
        };

        let src = &self.sources.read().unwrap()[&module_id];

        let position = params.text_document_position_params.position;
        let Some((symbol_id, loc)) = self.find_symbol(position, module_id) else {
            return Ok(None);
        };
        let symbols = self.symbols();
        let symbol = symbols.get_symbol(symbol_id);
        let type_display = self.display_signature(symbol_id);

        let docs: &str = symbol.docs().map_or("", |d| &d);

        let contents = HoverContents::Scalar(MarkedString::String(format!(
            r#"```tine
{}
```
---

{}
"#,
            type_display, docs
        )));

        Ok(Some(Hover {
            contents,
            range: Some(span_to_range(src, loc.span())),
        }))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}
