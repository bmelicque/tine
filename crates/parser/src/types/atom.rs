use tine_ast::*;
use tine_common::locations::Location;

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_atomic_type(&mut self) -> Option<Type> {
        match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_named_type().into()),
            Some((Ok(Token::LParen), _)) => Some(self.parse_tuple_type().into()),
            _ => None,
        }
    }

    pub fn parse_named_type(&mut self) -> NamedType {
        let Some((Ok(Token::Ident(name)), range)) = self.tokens.next() else {
            panic!()
        };
        let name = Identifier {
            loc: self.localize(range),
            text: name,
        };

        let (args, loc) = match self.maybe_parse_generic_args() {
            Some((args, loc)) => (Some(args), Location::merge(name.loc, loc)),
            None => (None, name.loc),
        };

        NamedType { loc, name, args }
    }
}
