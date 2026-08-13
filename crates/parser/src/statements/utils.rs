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
impl Locatable for TypeName {
    fn loc(&self) -> Location {
        self.loc
    }
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
}
