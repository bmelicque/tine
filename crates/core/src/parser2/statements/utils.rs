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

    /// Tries to parse a type name with its params.
    ///
    /// If there is no identifier, it will report a missing name error.
    ///
    /// If there is an identifier, returns `Ok(Some(..))`.
    /// If there is no identifier but the next expected token, returns `Ok(None)`.
    /// If there is no identifier and the next token is not in the expected list, returns `Err(..)`.
    pub(super) fn parse_type_name(&mut self, then: &[Token]) -> Result<Option<TypeName>, ()> {
        let name = match self.tokens.peek() {
            Some((Ok(Token::Ident(text)), range)) => {
                let text = text.to_owned();
                let range = range.clone();
                let loc = self.localize(range);
                ast::Identifier { loc, text }
            }
            Some((Ok(token), _)) if then.contains(token) => {
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

        let params = if let Some((Ok(Token::Lt), _)) = self.tokens.peek() {
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

    pub(super) fn parse_type_body(&mut self) -> Option<ast::TypeBody> {
        match self.tokens.peek() {
            Some((Ok(Token::LBrace), _)) => Some(self.parse_struct_body().into()),
            Some((Ok(Token::LParen), _)) => Some(self.parse_tuple_type().into()),
            _ => return None,
        }
    }

    fn parse_struct_body(&mut self) -> ast::StructBody {
        let start_range = self.eat(&[Token::LBrace]);

        let fields = self.parse_list(
            |p| p.parse_struct_definition_field(),
            Token::Comma,
            Token::RParen,
        );

        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RBrace), r)) => r.clone(),
            _ => self.recover_at(&[Token::RBrace]),
        };
        let loc = self.localize(start_range.start..end_range.end);
        ast::StructBody { loc, fields }
    }

    fn parse_struct_definition_field(&mut self) -> Option<ast::StructDefinitionField> {
        unimplemented!()
    }
}
