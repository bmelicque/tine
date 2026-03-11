use std::{collections::HashMap, sync::Arc};

use tine_core::{
    types::{FunctionType, GenericType, Type, TypeId},
    ModuleId, Source, SymbolData, SymbolKind, SymbolRef, TypeSymbolKind,
};
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenModifier, SemanticTokenType};

use crate::Backend;

#[derive(Debug, Clone)]
pub struct ServerSymbol(pub(crate) Arc<SymbolData>);

impl From<SymbolRef> for ServerSymbol {
    fn from(value: SymbolRef) -> Self {
        ServerSymbol(Arc::new(value.borrow().clone()))
    }
}
impl From<&SymbolRef> for ServerSymbol {
    fn from(value: &SymbolRef) -> Self {
        ServerSymbol(Arc::new(value.borrow().clone()))
    }
}

impl Backend {
    pub fn tokens_to_semantic(&self, id: ModuleId, src: &Source) -> Vec<SemanticToken> {
        let mut data = Vec::new();
        let mut map = HashMap::new();
        let symbols = self.session.read().unwrap().symbols();
        for symbol in &symbols {
            let symbol = ServerSymbol::from(symbol);
            map.insert(symbol.0.defined_at.span(), symbol.clone());
            symbol
                .0
                .access
                .uses()
                .filter(|l| l.module() == id)
                .map(|l| l.span())
                .for_each(|s| {
                    map.insert(s, symbol.clone());
                });
        }
        let mut tokens = map.into_iter().collect::<Vec<_>>();
        tokens.sort_by_key(|(span, _)| *span);

        let readonly_index = self
            .semantic_legend
            .token_modifiers
            .iter()
            .position(|m| *m == SemanticTokenModifier::READONLY)
            .unwrap();

        let mut prev_line = 0;
        let mut prev_col = 0;
        for (span, symbol) in tokens {
            let (start_line, start_col) = src.line_col(span.start());
            let (end_line, end_col) = src.line_col(span.end());
            let delta_line = start_line - prev_line;

            let delta_start = if delta_line == 0 {
                start_col - prev_col
            } else {
                start_col
            };

            let length = if start_line == end_line {
                end_col - start_col
            } else {
                // LSP semantic tokens must be single-line
                // Multi-line spans must be split beforehand
                continue;
            };

            let type_name = match symbol.0.kind {
                SymbolKind::Type { .. } => SemanticTokenType::TYPE,
                SymbolKind::Value { .. } => {
                    if let Type::Function(_) = self.get_type(symbol.0.ty) {
                        SemanticTokenType::FUNCTION
                    } else {
                        SemanticTokenType::VARIABLE
                    }
                }
                SymbolKind::Member { .. } => {
                    if let Type::Function(_) = self.get_type(symbol.0.ty) {
                        SemanticTokenType::METHOD
                    } else {
                        SemanticTokenType::PROPERTY
                    }
                }
                SymbolKind::Function { .. } => SemanticTokenType::FUNCTION,
                SymbolKind::Method { .. } => SemanticTokenType::METHOD,
                SymbolKind::Constructor { .. } => SemanticTokenType::ENUM_MEMBER,
            };
            let token_type_index = self
                .semantic_legend
                .token_types
                .iter()
                .position(|s| *s == type_name)
                .unwrap_or(0); // fallback

            let modifier_mask = if !symbol.0.is_mutable() {
                1 << readonly_index
            } else {
                0
            };

            data.push(SemanticToken {
                delta_line: delta_line as u32,
                delta_start: delta_start as u32,
                length: length as u32,
                token_type: token_type_index as u32,
                token_modifiers_bitset: modifier_mask,
            });

            prev_line = start_line;
            prev_col = start_col;
        }

        data
    }

    fn get_type(&self, id: TypeId) -> Type {
        let session = self.session.read().unwrap();
        let type_store = session.types();
        type_store.get(id).clone()
    }
}
