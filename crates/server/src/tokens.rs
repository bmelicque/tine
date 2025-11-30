use mylang_core::{
    types::{Type, TypeId},
    SymbolKind, TypeStore,
};
use tower_lsp::lsp_types::{Range, SemanticToken, SemanticTokenModifier, SemanticTokenType};

use crate::{Backend, ModuleSummary};

#[derive(Debug, Clone)]
pub struct ServerToken {
    pub name: String,
    pub range: Range,
    pub kind: SymbolKind,
    pub docs: Option<String>,
}

impl ServerToken {}

impl Backend {
    pub fn tokens_to_semantic(&self, summary: &ModuleSummary) -> Vec<SemanticToken> {
        let mut data = Vec::with_capacity(summary.tokens.len());
        let mut prev_line = 0;
        let mut prev_start = 0;

        let readonly_index = self
            .semantic_legend
            .token_modifiers
            .iter()
            .position(|m| *m == SemanticTokenModifier::READONLY)
            .unwrap();

        for token in &summary.tokens {
            let type_name = match token.kind {
                SymbolKind::Type(_) => SemanticTokenType::TYPE,
                SymbolKind::Value { .. } => SemanticTokenType::VARIABLE,
                SymbolKind::Function { .. } => SemanticTokenType::FUNCTION,
            };
            let token_type_index = self
                .semantic_legend
                .token_types
                .iter()
                .position(|s| *s == type_name)
                .unwrap_or(0); // fallback

            let modifier_mask = if !token.kind.is_mutable() {
                1 << readonly_index
            } else {
                0
            };

            let range = token.range;
            let start_line = range.start.line;
            let start_col = range.start.character;
            let length = (range.end.character - range.start.character) as u32;

            // delta encoding
            let delta_line = if data.is_empty() {
                start_line
            } else {
                start_line - prev_line
            };

            let delta_start = if delta_line == 0 {
                start_col - prev_start
            } else {
                start_col
            };

            data.push(SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type: token_type_index as u32,
                token_modifiers_bitset: modifier_mask,
            });

            prev_line = start_line;
            prev_start = start_col;
        }

        data
    }
}

pub fn display_signature(store: &TypeStore, name: &String, signature: SymbolKind) -> String {
    match signature {
        SymbolKind::Function { params, ty } => {
            let Type::Function(f) = store.get(ty) else {
                panic!()
            };
            let params = f
                .params
                .iter()
                .zip(params)
                .map(|(ty, name)| display_signature_param(store, &name, *ty))
                .collect::<Vec<_>>()
                .join(", ");
            match store.get(f.return_type) {
                Type::Unit | Type::Void => format!("{}({})", name, params),
                _ => format!(
                    "{}({}) => {}",
                    name,
                    params,
                    store.display_type(f.return_type)
                ),
            }
        }
        SymbolKind::Type(ty) => {
            let ty = store.display_raw_type(ty);
            format!("{} :: {}", name, ty)
        }
        SymbolKind::Value { ty, mutable } => {
            let ty = store.display_type(ty);
            let operator = if mutable { ":=" } else { "::" };
            format!("{} {} {}(..)", name, operator, ty)
        }
    }
}
fn display_signature_param(store: &TypeStore, name: &String, ty: TypeId) -> String {
    let ty = store.display_type(ty);
    format!("{} {}", name, ty)
}
