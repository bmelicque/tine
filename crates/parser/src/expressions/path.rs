use tine_ast::*;
use tine_common::locations::Location;

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub(crate) fn parse_path_expression(&mut self) -> PathExpression {
        let mut segments = Vec::new();
        loop {
            let (segment, has_next) = self.parse_path_expr_segment();
            segments.push(segment);
            if !has_next {
                break segments.into();
            }
        }
    }

    /// Parse a path segment. Also returns a `bool` equal to `true` if another
    /// `PathSegment` is to be expected after this one.
    fn parse_path_expr_segment(&mut self) -> (PathSegment, bool) {
        let identifier = self.parse_identifier();
        let Some(_) = self.maybe_eat(|t| t.dot()) else {
            return (identifier.into(), false);
        };
        let Some((args, end)) = self.maybe_parse_generic_args() else {
            return (identifier.into(), true);
        };
        let loc = Location::merge(identifier.loc, end);
        let has_next = self.maybe_eat(|t| t.dot()).is_some();
        let segment = PathSegment {
            loc,
            ident: identifier,
            generic_args: Some(args),
        };
        (segment, has_next)
    }

    pub fn maybe_parse_generic_args(&mut self) -> Option<(Vec<Type>, Location)> {
        self.maybe_eat(|t| t.lcaret())?;
        let generic_args = self.parse_list(|p| p.parse_type(), Token::Comma, Token::Gt);
        let end = self.expect(Token::Gt);
        let end = self.localize(end);
        Some((generic_args, end))
    }
}
