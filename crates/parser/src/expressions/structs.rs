use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub(crate) fn parse_struct_expression(
        &mut self,
        constructor: PathExpression,
    ) -> StructExpression {
        self.eat(&[Token::LBrace]);
        let fields = self.parse_list(|p| p.parse_struct_expr_field(), Token::Comma, Token::RBrace);
        let end_range = self.expect(Token::RBrace);
        let loc = Location::merge(constructor.loc(), self.localize(end_range));
        StructExpression {
            loc,
            constructor,
            fields,
        }
    }

    fn parse_struct_expr_field(&mut self) -> Option<StructExprField> {
        let key = self.parse_struct_expr_field_key();

        let Some((_, colon_loc)) = self.maybe_eat(|t| t.colon()) else {
            return key.map(|k| StructExprField {
                loc: k.loc(),
                key: Some(k),
                value: None,
            });
        };

        let value = self.parse_expression();
        if value.is_none() {
            let error = DiagnosticKind::MissingExpression;
            let error_loc = self.next_loc();
            self.error(error, error_loc);
        }

        let loc = match (&key, &value) {
            (Some(k), Some(v)) => Location::merge(k.loc(), v.loc()),
            (Some(k), None) => Location::merge(k.loc(), colon_loc),
            (None, Some(v)) => Location::merge(colon_loc, v.loc()),
            (None, None) => colon_loc,
        };

        Some(StructExprField { loc, key, value })
    }

    fn parse_struct_expr_field_key(&mut self) -> Option<StructExprFieldKey> {
        match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_identifier().into()),
            Some((Ok(Token::String(_) | Token::Int(_) | Token::Float(_) | Token::Bool(_)), _)) => {
                self.parse_atom().map(|a| a.into())
            }
            Some((Ok(Token::LBracket), _)) => {
                self.eat(&[Token::LBracket]);
                let expr = self.parse_expression();
                if expr.is_none() {
                    let loc = self.next_loc();
                    self.error(DiagnosticKind::MissingExpression, loc);
                }
                self.expect(Token::RBracket);
                expr.map(Into::into)
            }
            _ => None,
        }
    }
}
