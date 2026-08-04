use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

pub(super) struct TypeName {
    pub name: ast::Identifier,
    pub params: Option<Vec<ast::Identifier>>,
    pub loc: Location,
}

impl Parser<'_> {
    pub(crate) fn maybe_parse_name<R>(&mut self, recover_at: R) -> Option<ast::Identifier>
    where
        R: Fn(&Token) -> bool,
    {
        let name_result = self.try_parse(
            |self_| {
                let (text, loc) = self_.maybe_eat(|t| t.identifier().map(|s| s.to_owned()))?;
                Some(ast::Identifier { text, loc })
            },
            recover_at,
        );

        match name_result {
            Ok(Some(name)) => Some(name),
            Ok(None) => {
                let loc = self.next_loc();
                self.error(DiagnosticKind::MissingName, loc);
                None
            }
            Err(loc) => {
                self.error(DiagnosticKind::MissingName, loc);
                None
            }
        }
    }

    pub(super) fn try_parse_type_name(
        &mut self,
    ) -> (Option<ast::Identifier>, Option<Vec<ast::Identifier>>) {
        let name = match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => self.parse_identifier(),
            _ => {
                self.report_missing(DiagnosticKind::MissingName);
                return (None, None);
            }
        };

        let params = if let Some((Ok(Token::Lt), _)) = self.tokens.peek() {
            Some(self.parse_type_params())
        } else {
            None
        };

        (Some(name), params)
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
                self.tokens.next();
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
            Some((Ok(Token::LParen), _)) => Some(self.parse_tuple_body().into()),
            _ => return None,
        }
    }

    fn parse_struct_body(&mut self) -> ast::StructBody {
        let start_range = self.eat(&[Token::LBrace]);

        let fields = self.parse_list(
            |p| p.parse_struct_definition_field(),
            Token::Comma,
            Token::RBrace,
        );

        let end_range = self.expect(Token::RBrace);
        let loc = self.localize(start_range.start..end_range.end);
        ast::StructBody { loc, fields }
    }

    fn parse_struct_definition_field(&mut self) -> Option<ast::StructDefinitionField> {
        let (_, pub_loc) = self.maybe_eat(|t| t.pub_()).unzip();

        let name = self.maybe_parse_name(|t| {
            matches!(
                t,
                Token::Comma | Token::Colon | Token::Newline | Token::RBrace
            )
        })?;

        let colon = self.better_expect(
            |t| t.colon(),
            &[Token::Newline, Token::Comma, Token::RBrace],
        );
        if let Err(skipped) = colon {
            let error = DiagnosticKind::ExpectedToken {
                expected: vec![":".into()],
            };
            let loc = self.localize(skipped);
            self.error(error, loc);
            return Some(ast::StructDefinitionField {
                loc: pub_loc.map_or(name.loc, |l| Location::merge(l, name.loc)),
                name: Some(name),
                definition: None,
                public: pub_loc.is_some(),
            });
        }

        let definition = self.parse_type();
        if definition.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingType, loc);
        }

        let loc = match (pub_loc, &definition) {
            (Some(p), Some(def)) => Location::merge(p, def.loc()),
            (Some(p), None) => Location::merge(p, name.loc),
            (None, Some(def)) => Location::merge(name.loc, def.loc()),
            _ => return None,
        };

        Some(ast::StructDefinitionField {
            loc,
            name: Some(name),
            definition,
            public: pub_loc.is_some(),
        })
    }

    fn parse_tuple_body(&mut self) -> ast::TupleBody {
        let start_range = self.eat(&[Token::LParen]);

        let elements = self.parse_list(
            |parser| parser.parse_tuple_body_element(),
            Token::Comma,
            Token::RParen,
        );

        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), r)) => r.clone(),
            _ => self.recover_at(&[Token::RParen]),
        };

        ast::TupleBody {
            loc: self.localize(start_range.start..end_range.end),
            elements,
        }
    }

    fn parse_tuple_body_element(&mut self) -> Option<(bool, ast::Type)> {
        let is_public = self.eat_if(&[Token::Pub]).is_some();
        let ty = self.parse_type()?;
        Some((is_public, ty))
    }
}
