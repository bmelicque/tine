mod tokens;
mod utils;

use dashmap::DashMap;
use mylang_core::{analyze, ParseError};
use mylang_core::{Module, Token};
use std::path::PathBuf;
use std::sync::Arc;
use swc_common::FileName;
use tower_lsp::jsonrpc::Result;
use tower_lsp::Client;
use tower_lsp::{lsp_types::*, LspService, Server};
use url::Url;

use crate::utils::normalize_file_url;

#[derive(Debug, Clone)]
struct ModuleSummary {
    type_store: mylang_core::TypeStore,
    pub uri: Url,
    pub diagnostics: Vec<Diagnostic>,
    pub tokens: Vec<(Range, mylang_core::types::TypeId)>,
}

#[derive(Clone)]
struct Backend {
    client: Client,
    analyzed: Arc<DashMap<Url, ModuleSummary>>,
    semantic_legend: SemanticTokensLegend,
}

#[tower_lsp::async_trait]
impl tower_lsp::LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        let root = params.root_uri.and_then(|u| u.to_file_path().ok());

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                        SemanticTokensRegistrationOptions {
                            text_document_registration_options: TextDocumentRegistrationOptions {
                                document_selector: Some(vec![DocumentFilter {
                                    language: Some("my-lang".into()),
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
        let uri = params.text_document.uri;
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

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Ok(path) = uri.to_file_path() {
            self.run_project_analysis(path).await;
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = normalize_file_url(&params.text_document.uri).unwrap();
        let Some(summary) = self.analyzed.get(&uri) else {
            return Ok(None);
        };

        let data = self.tokens_to_semantic(&summary);

        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        let semantic_legend = SemanticTokensLegend {
            token_types: vec![
                SemanticTokenType::KEYWORD,
                SemanticTokenType::VARIABLE,
                SemanticTokenType::FUNCTION,
            ],
            token_modifiers: vec![],
        };
        Self {
            semantic_legend,
            client,
            analyzed: Arc::new(DashMap::new()),
        }
    }

    async fn run_project_analysis(&self, entry_path: PathBuf) {
        let client = self.client.clone();
        match get_summary(entry_path) {
            Ok(summary) => {
                for module in summary.iter() {
                    self.analyzed
                        .insert(normalize_file_url(&module.uri).unwrap(), module.clone());
                    client
                        .publish_diagnostics(module.uri.clone(), module.diagnostics.clone(), None)
                        .await;
                    client
                        .log_message(
                            MessageType::INFO,
                            format!(
                                "Analysis complete for entry {}, found {} error(s)",
                                module.uri,
                                module.diagnostics.len()
                            ),
                        )
                        .await;
                    let _ = self.client.semantic_tokens_refresh().await;
                }
            }
            Err(err) => {
                client
                    .log_message(
                        MessageType::ERROR,
                        format!("Analysis task failed: {:?}", err),
                    )
                    .await;
            }
        }
    }
}

fn get_summary(entry_point: PathBuf) -> anyhow::Result<Vec<ModuleSummary>, anyhow::Error> {
    let analyzed_modules = analyze(entry_point)?;
    Ok(analyzed_modules
        .modules
        .into_iter()
        .filter(|m| matches!(*m.borrow().name, FileName::Real(_)))
        .map(|m| summarize_module(&m.borrow()))
        .collect())
}

fn summarize_module(m: &Module) -> ModuleSummary {
    let uri: Url = match (*m.name).clone() {
        FileName::Real(path) => Url::from_file_path(path).unwrap(),
        _ => unreachable!(),
    };
    let diagnostics = m.errors.iter().map(|e| error_to_lsp(e)).collect();
    let Some(c) = &m.context else {
        unreachable!();
    };

    let tokens = c
        .tokens
        .iter()
        .map(|(span, t)| {
            let ty = match t {
                Token::Member(t) => t.ty,
                Token::Symbol(t) => t.symbol.borrow().ty,
            };
            (span_to_range(*span), ty)
        })
        .collect();

    ModuleSummary {
        type_store: c.type_store.clone(),
        uri,
        diagnostics,
        tokens,
    }
}

fn error_to_lsp(e: &ParseError) -> Diagnostic {
    Diagnostic {
        range: span_to_range(e.span.clone()),
        message: e.message.clone(),
        severity: Some(DiagnosticSeverity::ERROR),
        ..Default::default()
    }
}

fn span_to_range(span: pest::Span) -> Range {
    let (start_line, start_col) = span.start_pos().line_col();
    let (end_line, end_col) = span.end_pos().line_col();
    Range {
        start: Position::new((start_line - 1) as u32, (start_col - 1) as u32),
        end: Position::new((end_line - 1) as u32, (end_col - 1) as u32),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
