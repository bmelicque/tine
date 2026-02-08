use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind,
};

impl Parser<'_> {
    pub fn parse_atomic_type(&mut self) -> Option<ast::Type> {
        match self.tokens.peek() {
            Some((Ok(Token::Ident(_)), _)) => Some(self.parse_named_type().into()),
            Some((Ok(Token::LParen), _)) => Some(self.parse_tuple_type().into()),
            _ => None,
        }
    }

    fn parse_named_type(&mut self) -> ast::NamedType {
        let Some((Ok(Token::Ident(name)), mut range)) = self.tokens.next() else {
            panic!()
        };

        let args = if let Some((Ok(Token::Lt), _)) = self.tokens.peek() {
            let (args, end) = self.parse_type_args();
            range.end = end;
            Some(args)
        } else {
            None
        };

        ast::NamedType {
            loc: self.localize(range),
            name,
            args,
        }
    }

    pub(super) fn parse_type_args(&mut self) -> (Vec<ast::Type>, usize) {
        self.eat(&[Token::Lt]);
        let args = self.parse_list(|p| p.parse_type(), Token::Comma, Token::Gt);
        let result = self.better_expect(
            |t| match t {
                Token::Gt => Some(()),
                _ => None,
            },
            &[],
        );
        let end = match result {
            Ok((_, range)) => range.end.clone(),
            Err(range) => {
                let error = DiagnosticKind::ExpectedToken {
                    expected: vec![Token::Gt.to_string()],
                };
                let loc = self.localize(range.clone());
                self.error(error, loc);
                if let Some((Ok(Token::Gt), range)) = self.tokens.peek() {
                    let range = range.clone();
                    self.tokens.next();
                    range.end
                } else {
                    range.end
                }
            }
        };
        (args, end)
    }
}
