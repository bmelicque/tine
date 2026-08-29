use std::collections::HashMap;

use tine_common::{module_path::ModuleId, sources::Source};
use tine_symbols::symbols::*;
use tine_types::types::{Type, TypeId};
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

use crate::Backend;

impl Backend {
    pub fn tokens_to_semantic(&self, id: ModuleId, src: &Source) -> Vec<SemanticToken> {
        let symbols = self.symbols();
        let mut data = Vec::new();
        let mut map = HashMap::new();
        for symbol_id in symbols.all_ids() {
            let symbol = symbols.get_symbol(symbol_id);
            let defined_at = symbol.defined_at();
            if defined_at.module() == id {
                map.insert(symbol.defined_at().span(), symbol_id);
            }
            symbol
                .uses()
                .filter(|l| l.module() == id)
                .map(|l| l.span())
                .for_each(|s| {
                    map.insert(s, symbol_id);
                });
        }
        let mut tokens = map.into_iter().collect::<Vec<_>>();
        tokens.sort_by_key(|(span, _)| *span);

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

            let type_name = match symbol {
                SymbolId::Enum(_)
                | SymbolId::Primitive(_)
                | SymbolId::TypeAlias(_)
                | SymbolId::Struct(_) => SemanticTokenType::TYPE,
                SymbolId::Variable(s) => {
                    let ty = symbols.get(s).ty;
                    if let Type::Function(_) = self.get_type(ty) {
                        SemanticTokenType::FUNCTION
                    } else {
                        SemanticTokenType::VARIABLE
                    }
                }
                SymbolId::Member(s) => {
                    let ty = symbols.get(s).ty;
                    if let Type::Function(_) = self.get_type(ty) {
                        SemanticTokenType::METHOD
                    } else {
                        SemanticTokenType::PROPERTY
                    }
                }
                SymbolId::Function(_) => SemanticTokenType::FUNCTION,
                SymbolId::Method(_) => SemanticTokenType::METHOD,
                SymbolId::Trait(_) => SemanticTokenType::INTERFACE,
                SymbolId::Variant(_) => SemanticTokenType::ENUM_MEMBER,
            };
            let token_type_index = self
                .semantic_legend
                .token_types
                .iter()
                .position(|s| *s == type_name)
                .unwrap_or(0); // fallback

            data.push(SemanticToken {
                delta_line: delta_line as u32,
                delta_start: delta_start as u32,
                length: length as u32,
                token_type: token_type_index as u32,
                token_modifiers_bitset: 0,
            });

            prev_line = start_line;
            prev_col = start_col;
        }

        data
    }

    fn get_type(&self, id: TypeId) -> Type {
        let type_store = self.types();
        type_store.get(id).clone()
    }
}
