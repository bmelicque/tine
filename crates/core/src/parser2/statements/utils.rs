use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

pub(super) struct TypeName {
    pub name: ast::Identifier,
    pub params: Option<Vec<String>>,
    pub loc: Location,
}

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

    pub(super) fn parse_type_name(&mut self, then: Token) -> Result<Option<TypeName>, ()> {
        let name = match self.tokens.peek() {
            Some((Ok(Token::Ident(text)), range)) => {
                let text = text.to_owned();
                let range = range.clone();
                let loc = self.localize(range);
                ast::Identifier { loc, text }
            }
            Some((Ok(token), _)) if *token == then => {
                let error_loc = self.next_loc();
                self.error(DiagnosticKind::MissingName, error_loc);
                return Ok(None);
            }
            _ => {
                let error_loc = self.next_loc();
                self.error(DiagnosticKind::MissingName, error_loc);
                return Err(());
            }
        };

        let params = if let Some((Ok(Token::Lt), range)) = self.tokens.peek() {
            Some(self.parse_type_params())
        } else {
            None
        };

        let loc = match &params {
            Some(_) => Location::merge(name.loc, self.next_loc().decrement()),
            None => name.loc,
        };

        Ok(Some(TypeName { name, params, loc }))
    }
}
