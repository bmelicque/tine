use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    const TYPE_BINARY_OPERATORS: [Token; 2] = [Token::Bang, Token::Hash];

    pub fn parse_binary_type(&mut self, min_precedence: u8) -> Option<ast::Type> {
        let highest_precedence = Self::TYPE_BINARY_OPERATORS
            .iter()
            .fold(0, |max, cur| std::cmp::max(max, cur.precedence()));
        if min_precedence > highest_precedence {
            return self.parse_unary_type();
        }
        let mut ty = self.parse_binary_type(min_precedence + 1);
        while let Some((Ok(token), op_range)) = self.tokens.peek().cloned() {
            if token.precedence() < min_precedence || !Self::TYPE_BINARY_OPERATORS.contains(&token)
            {
                break;
            }
            self.tokens.next(); // consume the operator
            let right = self.parse_binary_type(min_precedence + 1);
            if right.is_none() {
                self.error(
                    DiagnosticKind::MissingType,
                    self.localize(op_range.clone()).increment(),
                );
            }

            let loc = match (&ty, &right) {
                (Some(l), Some(r)) => Location::merge(l.loc(), r.loc()),
                (Some(l), None) => Location::merge(l.loc(), self.localize(op_range)),
                (None, Some(r)) => Location::merge(self.localize(op_range), r.loc()),
                (None, None) => self.localize(op_range),
            };

            ty = Some(match token {
                Token::Bang => ast::Type::Result(ast::ResultType {
                    loc,
                    error: ty.map(|t| Box::new(t)),
                    ok: right.map(|t| Box::new(t)),
                }),
                Token::Hash => ast::Type::Map(ast::MapType {
                    loc,
                    key: ty.map(|t| Box::new(t)),
                    value: right.map(|t| Box::new(t)),
                }),
                // other tokens prevented by the check at the start of the loop
                _ => unreachable!(),
            });
        }
        ty
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use crate::test_utils::{test_type, TypeTest};

    use super::*;

    fn loc(start: u32, end: u32) -> Location {
        Location::new(0, Span::new(start, end))
    }

    #[test]
    fn parse_result_type() {
        test_type(TypeTest {
            input: "str!bool",
            expected: ast::Type::Result(ast::ResultType {
                loc: loc(0, 8),
                error: Some(Box::new(ast::Type::Named(
                    ast::Identifier::new("str".to_string(), loc(0, 3)).into(),
                ))),
                ok: Some(Box::new(ast::Type::Named(
                    ast::Identifier::new("bool".to_string(), loc(4, 8)).into(),
                ))),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_map_type() {
        test_type(TypeTest {
            input: "str#bool",
            expected: ast::Type::Map(ast::MapType {
                loc: loc(0, 8),
                key: Some(Box::new(ast::Type::Named(
                    ast::Identifier::new("str".to_string(), loc(0, 3)).into(),
                ))),
                value: Some(Box::new(ast::Type::Named(
                    ast::Identifier::new("bool".to_string(), loc(4, 8)).into(),
                ))),
            }),
            diagnostics: vec![],
        });
    }
}
