mod display;
mod loader;
mod lsp;
mod tokens;
mod utils;

use std::collections::HashMap;
use std::ffi::OsString;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tine_core::symbols::SymbolTable;
use tine_core::type_store::TypeStore;
use tine_core::{Diagnostic as ParserDiagnostic, ModulePath};
use tine_core::{ModuleId, Source, Span};
use tower_lsp::Client;
use tower_lsp::{lsp_types::*, LspService, Server};
use url::Url;

use crate::loader::LspLoader;

#[derive(Clone)]
struct Backend {
    client: Client,
    ids: Arc<RwLock<HashMap<ModulePath, ModuleId>>>,
    sources: Arc<RwLock<HashMap<ModuleId, Source>>>,
    types: Arc<RwLock<TypeStore>>,
    symbols: Arc<RwLock<SymbolTable>>,
    semantic_legend: SemanticTokensLegend,
    open_files: Arc<RwLock<HashMap<Url, String>>>,
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
            ids: Arc::new(RwLock::new(HashMap::new())),
            sources: Arc::new(RwLock::new(HashMap::new())),
            types: Arc::new(RwLock::new(TypeStore::new())),
            symbols: Arc::new(RwLock::new(SymbolTable::default())),
            open_files,
        }
    }

    pub fn loader(&self) -> LspLoader {
        LspLoader::new(self.open_files.clone())
    }

    async fn run_project_analysis(&self, entry_path: PathBuf) {
        let client = self.client.clone();

        let module_path = ModulePath::from(&entry_path);
        let loader = self.loader();
        let parse_result = tine_core::parse_project(module_path.clone(), Some(Box::new(loader)));
        let result = tine_core::check_project(parse_result);
        {
            let mut types = self.types.write().unwrap();
            *types = result.types;
            let mut symbols = self.symbols.write().unwrap();
            *symbols = result.symbols;
        }

        let diagnostics = result.diagnostics;
        let diagnostics = diagnostics.into_iter().filter_map(|(id, diags)| {
            let ModulePath::Real(name) = result.names[id].clone() else {
                return None;
            };
            let uri = Url::from_file_path(name).unwrap();
            let len = diags.len();
            let diags = diags
                .iter()
                .map(|diag| error_to_lsp(&result.sources[&id], diag))
                .collect::<Vec<_>>();
            Some((uri, diags, len))
        });

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

    fn find_module(&self, uri: &Url) -> Option<ModuleId> {
        let path = PathBuf::from(OsString::from(uri.path()))
            .canonicalize()
            .ok()?;
        let ids = self.ids.read().unwrap();
        ids.get(&ModulePath::Real(path)).copied()
    }

    fn symbols(&self) -> std::sync::RwLockReadGuard<'_, SymbolTable> {
        self.symbols.read().unwrap()
    }

    fn types(&self) -> std::sync::RwLockReadGuard<'_, TypeStore> {
        self.types.read().unwrap()
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
