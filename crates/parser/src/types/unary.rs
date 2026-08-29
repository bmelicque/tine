use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_unary_type(&mut self) -> Option<Type> {
        let mut ty = self.parse_atomic_type();

        while let Some((Ok(token), _)) = self.tokens.peek() {
            match token {
                Token::LBracket => {
                    ty = Some(self.parse_array_type(ty).into());
                }
                Token::QMark => {
                    ty = Some(Type::Option(OptionType {
                        loc: self.unary_type_loc(&ty),
                        base: ty.map(Box::new),
                    }))
                }
                _ => break,
            }
        }
        ty
    }

    fn parse_array_type(&mut self, element: Option<Type>) -> ArrayType {
        if element.is_none() {
            let loc = self.next_loc();
            self.error(DiagnosticKind::MissingType, loc);
        }
        let a = self.parse_array();
        if !a.elements.is_empty() {
            self.error(DiagnosticKind::UnexpectedExpression, a.loc);
        }
        let loc = element
            .as_ref()
            .map_or(a.loc, |t| Location::merge(t.loc(), a.loc));
        ArrayType {
            loc,
            element: element.map(Box::new),
        }
    }

    fn unary_type_loc(&mut self, inner: &Option<Type>) -> Location {
        let r = self.tokens.next().unwrap().1;
        let end = self.localize(r);
        match inner {
            Some(i) => Location::merge(i.loc(), end),
            None => end,
        }
    }
}
