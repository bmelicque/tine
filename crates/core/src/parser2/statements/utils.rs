use crate::{
    parser2::{tokens::Token, Parser},
    DiagnosticKind,
};

impl Parser<'_> {
    pub(super) fn try_parse_type_name(&mut self) -> Option<(String, Option<Vec<String>>)> {
        let name = match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => {
                let Some((Ok(Token::Ident(name)), _)) = self.tokens.next() else {
                    unreachable!()
                };
                name
            }
            _ => {
                self.report_missing(DiagnosticKind::MissingName);
                return None;
            }
        };

        let params = if let Some((Ok(Token::Lt), _)) = self.tokens.peek() {
            Some(self.parse_type_params())
        } else {
            None
        };

        Some((name, params))
    }
}
