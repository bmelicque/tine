use tine_ast::*;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub(crate) fn parse_path_expression(&mut self) -> PathExpression {
        let (first, next) = self.parse_path_expr_segment().unwrap();
        let mut segments = vec![first];
        let Some(mut next) = next else {
            return segments.into();
        };
        loop {
            let Some((segment, dot_loc)) = self.parse_path_expr_segment() else {
                self.error(DiagnosticKind::MissingExpression, next.increment());
                break segments.into();
            };
            segments.push(segment);
            match dot_loc {
                Some(loc) => next = loc,
                None => break segments.into(),
            }
        }
    }

    /// Parse a path segment
    fn parse_path_expr_segment(&mut self) -> Option<(PathSegment, Option<Location>)> {
        let identifier = self.maybe_parse_identifier()?;
        let Some((_, dot_loc)) = self.maybe_eat(|t| t.dot()) else {
            return Some((identifier.into(), None));
        };
        let Some((args, end)) = self.maybe_parse_generic_args() else {
            return Some((identifier.into(), Some(dot_loc)));
        };
        let loc = Location::merge(identifier.loc, end);
        let has_next = self.maybe_eat(|t| t.dot()).map(|t| t.1);
        let segment = PathSegment {
            loc,
            ident: identifier,
            generic_args: Some(args),
        };
        Some((segment, has_next))
    }

    pub fn maybe_parse_generic_args(&mut self) -> Option<(Vec<Type>, Location)> {
        self.maybe_eat(|t| t.lcaret())?;
        let generic_args = self.parse_list(|p| p.parse_type(), Token::Comma, Token::Gt);
        let end = self.expect(Token::Gt);
        let end = self.localize(end);
        Some((generic_args, end))
    }
}
