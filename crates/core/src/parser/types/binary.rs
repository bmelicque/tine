use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    const TYPE_BINARY_OPERATORS: [Token; 2] = [Token::Bang, Token::Hash];

    pub fn parse_binary_type(&mut self, min_precedence: u8) -> Option<ast::Type> {
        if min_precedence == Token::Hash.precedence() {
            return self.parse_unary_type();
        }
        let mut ty = self.parse_binary_type(min_precedence + 1);
        while let Some((Ok(token), op_range)) = self.tokens.peek().cloned() {
            if token.precedence() <= min_precedence || !Self::TYPE_BINARY_OPERATORS.contains(&token)
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
