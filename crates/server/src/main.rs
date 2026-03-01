mod loader;
mod tokens;
mod utils;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tine_core::{analyze, ModuleId, Session, Source, Span};
use tine_core::{Diagnostic as ParserDiagnostic, ModulePath};
use tower_lsp::jsonrpc::Result;
use tower_lsp::Client;
use tower_lsp::{lsp_types::*, LspService, Server};
use url::Url;

use crate::loader::LspLoader;
use crate::utils::normalize_file_url;

#[derive(Debug, Clone)]
pub struct ModuleSummary {
    pub id: ModuleId,
    pub uri: Url,
    pub src: Source,
    pub diagnostics: Vec<ParserDiagnostic>,
}

#[derive(Clone)]
struct Backend {
    client: Client,
    session: Arc<RwLock<Session>>,
    semantic_legend: SemanticTokensLegend,
    open_files: Arc<RwLock<HashMap<Url, String>>>,
}

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
        let session = self.session.read().unwrap();
        let src = &session.read_module(module_id).src;
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

        let session = self.session.read().unwrap();
        let src = &session.read_module(module_id).src;

        let position = params.text_document_position_params.position;
        for symbol in &session.symbols() {
            for loc in symbol.uses().iter().filter(|l| l.module() == module_id) {
                if position_in_span(src, loc.span(), position) {
                    let type_display = self.display_signature(&symbol.into());

                    let docs = symbol.borrow().docs.clone().unwrap_or("".into());

                    let contents = HoverContents::Scalar(MarkedString::String(format!(
                        r#"```tine
{}
```
---

{}
"#,
                        type_display, docs
                    )));

                    return Ok(Some(Hover {
                        contents,
                        range: Some(span_to_range(src, loc.span())),
                    }));
                }
            }
        }
        return Ok(None);
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
                SemanticTokenType::TYPE,
                SemanticTokenType::VARIABLE,
                SemanticTokenType::FUNCTION,
                SemanticTokenType::METHOD,
                SemanticTokenType::ENUM_MEMBER,
                SemanticTokenType::PROPERTY,
            ],
            token_modifiers: vec![SemanticTokenModifier::READONLY],
        };
        let open_files = Arc::new(RwLock::new(HashMap::new()));
        Self {
            semantic_legend,
            client,
            session: Arc::new(RwLock::new(Session::new(Box::new(LspLoader::new(
                open_files.clone(),
            ))))),
            open_files,
        }
    }

    pub fn loader(&self) -> LspLoader {
        LspLoader::new(self.open_files.clone())
    }

    async fn run_project_analysis(&self, entry_path: PathBuf) {
        let client = self.client.clone();

        {
            let mut session = self.session.write().unwrap();
            let loader = self.loader();
            *session = analyze(entry_path.into(), Box::new(loader));
        }

        let diagnostics = self.get_diagnostics();
        for (uri, diags, len) in diagnostics {
            client.publish_diagnostics(uri.clone(), diags, None).await;
            client
                .log_message(
                    MessageType::INFO,
                    format!(
                        "Analysis complete for entry {}, found {} error(s)",
                        uri, len
                    ),
                )
                .await;
        }

        let _ = client.semantic_tokens_refresh().await;
    }

    fn get_diagnostics(&self) -> Vec<(Url, Vec<Diagnostic>, usize)> {
        let session = self.session.read().unwrap();
        session
            .diagnostics()
            .iter()
            .map(|(&m, diags)| {
                let module = session.read_module(m);
                let ModulePath::Real(name) = &module.name else {
                    return None;
                };

                let uri = Url::from_file_path(name).unwrap();
                let len = diags.len();
                let diags = diags
                    .iter()
                    .map(|diag| error_to_lsp(&module.src, diag))
                    .collect::<Vec<_>>();

                Some((uri, diags, len))
            })
            .flatten()
            .collect::<Vec<_>>()
    }

    fn find_module(&self, uri: &Url) -> Option<ModuleId> {
        let session = self.session.read().unwrap();
        session.modules().iter().position(|m| match &m.name {
            ModulePath::Real(path) => Url::from_file_path(path).unwrap() == *uri,
            _ => false,
        })
    }
}

fn position_in_span(src: &Source, span: Span, pos: Position) -> bool {
    let (start_line, start_col) = src.line_col(span.start());
    let (start_line, start_col) = (start_line as u32, start_col as u32);
    let (end_line, end_col) = src.line_col(span.end());
    let (end_line, end_col) = (end_line as u32, end_col as u32);
    if start_line > pos.line || end_line < pos.line {
        return false;
    }
    if start_line < pos.line && end_line > pos.line {
        return false;
    }
    if start_line == pos.line {
        return pos.character >= start_col;
    } else {
        return pos.character < end_col;
    }
}

fn error_to_lsp(src: &Source, e: &ParserDiagnostic) -> Diagnostic {
    Diagnostic {
        range: span_to_range(src, e.loc.span()),
        message: format!("{}", e.kind),
        severity: Some(DiagnosticSeverity::ERROR),
        ..Default::default()
    }
}

fn span_to_range(src: &Source, span: Span) -> Range {
    let (start_line, start_col) = src.line_col(span.start());
    let (end_line, end_col) = src.line_col(span.end());
    Range {
        start: Position::new(start_line as u32, start_col as u32),
        end: Position::new(end_line as u32, end_col as u32),
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let (stdin, stdout) = (tokio::io::stdin(), tokio::io::stdout());
    let (service, socket) = LspService::new(|client| Backend::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
    Ok(())
}
