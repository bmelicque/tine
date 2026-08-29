use tine_ast as ast;

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_tuple_type(&mut self) -> ast::TupleType {
        let start_range = self.eat(&[Token::LParen]);

        let elements = self.parse_list(|parser| parser.parse_type(), Token::Comma, Token::RParen);

        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), r)) => r.clone(),
            _ => self.recover_at(&[Token::RParen]),
        };
        self.eat_if(&[Token::RParen]);

        ast::TupleType {
            loc: self.localize(start_range.start..end_range.end),
            elements,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Parser;

    #[test]
    fn parse_function_type() {
        let mut parser = Parser::new(0, "(int, str)");
        let ty = parser.parse_type().expect("expected a type");
        let t = ty.as_tuple().expect("expected a tuple type");
        assert_eq!(t.elements.len(), 2);
    }
}
