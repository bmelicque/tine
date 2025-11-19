use mylang_core::types::Type;
use tower_lsp::lsp_types::{SemanticToken, SemanticTokenType};

use crate::{Backend, ModuleSummary};

impl Backend {
    pub fn tokens_to_semantic(&self, summary: &ModuleSummary) -> Vec<SemanticToken> {
        let tokens = {
            let mut tokens = summary.tokens.clone();
            tokens.sort_by(|(a, _), (b, _)| {
                (a.start.line, a.start.character).cmp(&(b.start.line, b.start.character))
            });
            tokens
        };

        let mut data = Vec::with_capacity(tokens.len());
        let mut prev_line = 0;
        let mut prev_start = 0;

        for (range, type_id) in tokens {
            let global_type = summary.type_store.get(type_id);

            let type_name = token_type_from_global(&global_type);
            let token_type_index = self
                .semantic_legend
                .token_types
                .iter()
                .position(|s| *s == type_name)
                .unwrap_or(0); // fallback

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

fn token_type_from_global(ty: &Type) -> SemanticTokenType {
    match ty {
        Type::Function(_) => SemanticTokenType::FUNCTION,
        _ => SemanticTokenType::VARIABLE,
    }
}
