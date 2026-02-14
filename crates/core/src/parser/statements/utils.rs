use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

pub(super) struct TypeName {
    pub name: ast::Identifier,
    pub params: Option<Vec<ast::Identifier>>,
    pub loc: Location,
}

impl Parser<'_> {
    pub(super) fn try_parse_type_name(&mut self) -> Option<(String, Option<Vec<ast::Identifier>>)> {
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
        let name = match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_identifier()),
            Some(_) => {
                let loc = self.next_loc();
                self.error(DiagnosticKind::MissingName, loc);
                None
            }
            None => {
                let loc = self.next_loc();
                self.error(DiagnosticKind::MissingName, loc);
                return None;
            }
        };

        match self.tokens.peek() {
            Some((Ok(Token::Eq), eq_range)) => {
                let eq_range = eq_range.clone();
                let eq_loc = self.localize(eq_range);
                self.tokens.next();

                let default = self.parse_expression();
                if default.is_none() {
                    let loc = self.next_loc();
                    self.error(DiagnosticKind::MissingExpression, loc);
                }

                let loc = match (&name, &default) {
                    (Some(name), Some(default)) => Location::merge(name.loc, default.loc()),
                    (Some(name), None) => Location::merge(name.loc, eq_loc),
                    (None, Some(default)) => Location::merge(eq_loc, default.loc()),
                    _ => eq_loc,
                };

                Some(ast::StructDefinitionField::Optional(
                    ast::StructOptionalField { loc, name, default },
                ))
            }
            _ => {
                let definition = self.parse_type();
                if definition.is_none() {
                    let loc = self.next_loc();
                    self.error(DiagnosticKind::MissingType, loc);
                }

                let loc = match (&name, &definition) {
                    (Some(name), Some(def)) => Location::merge(name.loc, def.loc()),
                    (Some(name), None) => name.loc,
                    (None, Some(def)) => def.loc(),
                    _ => return None,
                };

                Some(ast::StructDefinitionField::Mandatory(
                    ast::StructMandatoryField {
                        loc,
                        name,
                        definition,
                    },
                ))
            }
        }
    }
}
