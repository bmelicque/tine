use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    const UNARY_TYPE_OPERATORS: [Token; 5] = [
        Token::And,
        Token::At,
        Token::Bang,
        Token::LBracket,
        Token::QMark,
    ];

    pub fn parse_unary_type(&mut self) -> Option<ast::Type> {
        match self.tokens.peek() {
            Some((Ok(token), _)) if Self::UNARY_TYPE_OPERATORS.contains(token) => {}
            _ => return self.parse_atomic_type(),
        }

        let Some((Ok(token), op_range)) = self.tokens.next() else {
            unreachable!()
        };
        let mut op_loc = self.localize(op_range);
        if token == Token::LBracket {
            op_loc = self.parse_array_token(op_loc);
        }
        let inner = self.parse_unary_type();
        if inner.is_none() {
            self.error(DiagnosticKind::MissingExpression, op_loc.increment());
        }
        let loc = match &inner {
            Some(inner) => Location::merge(op_loc, inner.loc()),
            None => op_loc,
        };
        let ty = match token {
            Token::And => ast::Type::Reference(ast::ReferenceType {
                loc,
                target: inner.map(|t| Box::new(t)),
            }),
            Token::At => ast::Type::Listener(ast::ListenerType {
                loc,
                inner: inner.map(|t| Box::new(t)),
            }),
            Token::Bang => ast::Type::Result(ast::ResultType {
                loc,
                error: None,
                ok: inner.map(|t| Box::new(t)),
            }),
            Token::LBracket => ast::Type::Array(ast::ArrayType {
                loc,
                element: inner.map(|t| Box::new(t)),
            }),
            Token::QMark => ast::Type::Option(ast::OptionType {
                loc,
                base: inner.map(|t| Box::new(t)),
            }),
            // unreachable because tokens are filtered at the top of the function
            _ => unreachable!(),
        };
        Some(ty)
    }

    fn parse_array_token(&mut self, start_loc: Location) -> Location {
        let result = self.better_expect(
            |t| match t {
                Token::RBracket => Some(()),
                _ => None,
            },
            &[Token::Newline],
        );
        match result {
            Ok(((), range)) => Location::merge(start_loc, self.localize(range)),
            Err(error_range) => match self.tokens.peek() {
                Some((Ok(Token::RBracket), range)) => {
                    let range = range.clone();
                    let loc = self.localize(range);
                    Location::merge(start_loc, loc)
                }
                _ => {
                    let range = error_range.clone();
                    let loc = self.localize(range);
                    Location::merge(start_loc, loc)
                }
            },
        }
    }
}
