use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    Location,
};

impl Parser<'_> {
    pub fn parse_array(&mut self) -> ast::ArrayExpression {
        let start_range = self.eat(&[Token::LBracket]);

        let elements = self.parse_list(|p| p.parse_expression(), Token::Comma, Token::RBracket);

        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RBracket), r)) => r.clone(),
            _ => self.recover_at(&[Token::RBracket]),
        };

        ast::ArrayExpression {
            loc: Location::merge(self.localize(start_range), self.localize(end_range)),
            elements,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser2::test_utils::{test_expression, ExpressionTest},
        Span,
    };

    use super::*;

    #[test]
    fn parse_empty_array() {
        test_expression(ExpressionTest {
            input: "[]",
            expected: ast::Expression::Array(ast::ArrayExpression {
                loc: Location::new(0, Span::new(0, 2)),
                elements: vec![],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_array() {
        test_expression(ExpressionTest {
            input: "[1]",
            expected: ast::Expression::Array(ast::ArrayExpression {
                loc: Location::new(0, Span::new(0, 3)),
                elements: vec![ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(1, 2)),
                    value: 1,
                })],
            }),
            diagnostics: vec![],
        });
    }
}
