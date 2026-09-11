mod display;
mod loader;
mod lsp;
mod tokens;
mod utils;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tine_common::{
    diagnostics::Diagnostic as ParserDiagnostic,
    locations::Span,
    module_path::{ModuleId, ModulePath},
    sources::Source,
};
use tine_symbols::symbols::SymbolId;
use tine_symbols::table::SymbolTable;
use tine_types::store::TypeStore;
use tower_lsp::Client;
use tower_lsp::{lsp_types::*, LspService, Server};
use url::Url;

use crate::loader::LspLoader;
use crate::utils::normalize_file_url;

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
                SemanticTokenType::INTERFACE,
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
        let in_graph = self.ids.read().unwrap().keys().any(|k| *k == module_path);
        let module_path = match in_graph {
            true => self.find_graph_root(),
            false => module_path,
        };
        let loader = self.loader();
        let parse_result = tine_parser::parse_project(module_path.clone(), Some(Box::new(loader)));
        let result = tine_checker::check_project(parse_result);
        {
            let mut types = self.types.write().unwrap();
            *types = result.types;
            let mut symbols = self.symbols.write().unwrap();
            *symbols = result.symbols;
            let mut ids = self.ids.write().unwrap();
            *ids = result.ids;
            let mut src = self.sources.write().unwrap();
            *src = result.sources;
        }

        for (id, module) in result.names.iter().enumerate() {
            let ModulePath::Real(path) = module else {
                continue;
            };

            let uri = Url::from_file_path(path).unwrap();

            let diags = result
                .diagnostics
                .get(&(id + 1))
                .map(|src_diags| {
                    let source = &self.sources.read().unwrap()[&(id + 1)];
                    src_diags
                        .iter()
                        .map(|d| error_to_lsp(source, d))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            client.publish_diagnostics(uri, diags, None).await;
        }

        let _ = client.semantic_tokens_refresh().await;
    }

    fn find_graph_root(&self) -> ModulePath {
        self.ids
            .read()
            .unwrap()
            .iter()
            .find(|(_, id)| **id == 1)
            .unwrap()
            .0
            .clone()
    }

    fn find_module(&self, uri: &Url) -> Option<ModuleId> {
        let path = normalize_file_url(uri)?.to_file_path().ok()?;
        let ids = self.ids.read().unwrap();
        ids.get(&ModulePath::Real(path)).copied()
    }

    fn symbols(&self) -> std::sync::RwLockReadGuard<'_, SymbolTable> {
        self.symbols.read().unwrap()
    }

    fn find_symbol(
        &self,
        pos: Position,
        module: ModuleId,
    ) -> Option<(SymbolId, tine_common::locations::Location)> {
        let src = &self.sources.read().unwrap()[&module];
        let symbols = self.symbols();
        for symbol_id in symbols.all_ids() {
            let symbol = symbols.get_symbol(symbol_id);
            let defined_at = symbol.defined_at();
            if defined_at.module() == module && position_in_span(src, defined_at.span(), pos) {
                return Some((symbol_id, defined_at));
            }
            for loc in symbol.uses().filter(|l| l.module() == module) {
                if position_in_span(src, loc.span(), pos) {
                    return Some((symbol_id, loc));
                }
            }
        }
        return None;
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
        return pos.character >= start_col && pos.character < end_col;
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
