use crate::{
    ast,
    parser::{tokens::Token, Parser},
};

impl Parser<'_> {
    pub fn parse_tuple_type(&mut self) -> ast::TupleType {
        let start_range = self.eat(&[Token::LParen]);

        let elements = self.parse_list(|parser| parser.parse_type(), Token::Comma, Token::RParen);

        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), r)) => r.clone(),
            _ => self.recover_at(&[Token::RParen]),
        };

        ast::TupleType {
            loc: self.localize(start_range.start..end_range.end),
            elements,
        }
    }
}
