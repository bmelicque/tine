use mylang_core::{
    types::{Type, TypeId},
    SymbolKind,
};
use tower_lsp::lsp_types::{Range, SemanticToken, SemanticTokenType};

use crate::{Backend, ModuleSummary};

#[derive(Debug, Clone)]
pub struct ServerToken {
    pub range: Range,
    pub ty: TypeId,
    pub kind: SymbolKind,
}

impl Backend {
    pub fn tokens_to_semantic(&self, summary: &ModuleSummary) -> Vec<SemanticToken> {
        let mut data = Vec::with_capacity(summary.tokens.len());
        let mut prev_line = 0;
        let mut prev_start = 0;

        for token in &summary.tokens {
            let global_type = summary.type_store.get(token.ty);
            let type_name = match token.kind {
                SymbolKind::Type => SemanticTokenType::TYPE,
                SymbolKind::Value => match global_type {
                    Type::Function(_) => SemanticTokenType::FUNCTION,
                    _ => SemanticTokenType::VARIABLE,
                },
            };
            let token_type_index = self
                .semantic_legend
                .token_types
                .iter()
                .position(|s| *s == type_name)
                .unwrap_or(0); // fallback

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
                token_modifiers_bitset: 0, // token modifiers: none for now
            });

            prev_line = start_line;
            prev_start = start_col;
        }

        data
    }
}
