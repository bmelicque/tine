use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    const LTR_BINARY_OPERATORS: [Token; 13] = [
        Token::Plus,
        Token::Minus,
        Token::Star,
        Token::Slash,
        Token::Mod,
        Token::AndAnd,
        Token::PipePipe,
        Token::EqEq,
        Token::NotEq,
        Token::Lt,
        Token::Le,
        Token::Gt,
        Token::Ge,
    ];

    pub fn parse_binary_expression(&mut self, min_precedence: u8) -> Option<ast::Expression> {
        if min_precedence == Token::StarStar.precedence() {
            return self.parse_exponentiation();
        }
        let mut expression = self.parse_binary_expression(min_precedence + 1);
        while let Some((Ok(token), op_range)) = self.tokens.peek().cloned() {
            if token.precedence() < min_precedence || !Self::LTR_BINARY_OPERATORS.contains(&token) {
                break;
            }
            self.tokens.next(); // consume the operator
            let operator = token.to_string().into();
            let right = self.parse_binary_expression(min_precedence + 1);
            if right.is_none() {
                self.error(
                    DiagnosticKind::MissingExpression,
                    self.localize(op_range.clone()).increment(),
                );
            }
            let loc = match (&expression, &right) {
                (Some(lhs), Some(rhs)) => Location::merge(lhs.loc(), rhs.loc()),
                (Some(lhs), None) => Location::merge(lhs.loc(), self.localize(op_range)),
                (None, Some(rhs)) => Location::merge(self.localize(op_range), rhs.loc()),
                (None, None) => self.localize(op_range),
            };
            expression = Some(ast::Expression::Binary(ast::BinaryExpression {
                loc,
                left: expression.map(|e| Box::new(e)),
                operator,
                right: right.map(|r| Box::new(r)),
            }))
        }
        expression
    }

    fn parse_exponentiation(&mut self) -> Option<ast::Expression> {
        let lhs = self.parse_unary_expression();
        let Some((Ok(Token::StarStar), _)) = self.tokens.peek() else {
            return lhs;
        };
        let op_range = self.eat(&[Token::StarStar]);
        let rhs = self.parse_exponentiation();
        if rhs.is_none() {
            self.error(
                DiagnosticKind::MissingExpression,
                self.localize(op_range.clone()).increment(),
            );
        }
        let loc = match (&lhs, &rhs) {
            (Some(lhs), Some(rhs)) => Location::merge(lhs.loc(), rhs.loc()),
            (Some(lhs), None) => Location::merge(lhs.loc(), self.localize(op_range)),
            (None, Some(rhs)) => Location::merge(self.localize(op_range), rhs.loc()),
            (None, None) => self.localize(op_range),
        };
        Some(ast::Expression::Binary(ast::BinaryExpression {
            loc,
            left: lhs.map(|lhs| Box::new(lhs)),
            operator: ast::BinaryOperator::Pow,
            right: rhs.map(|rhs| Box::new(rhs)),
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::test_utils::{test_expression, ExpressionTest},
        Span,
    };

    use super::*;

    #[test]
    fn parse_binary_expression() {
        test_expression(ExpressionTest {
            input: "1 + 2",
            expected: ast::Expression::Binary(ast::BinaryExpression {
                loc: Location::new(0, Span::new(0, 5)),
                left: Some(Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(0, 1)),
                    value: 1,
                }))),
                operator: ast::BinaryOperator::Add,
                right: Some(Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(4, 5)),
                    value: 2,
                }))),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_binary_logical_expression() {
        test_expression(ExpressionTest {
            input: "true || false",
            expected: ast::Expression::Binary(ast::BinaryExpression {
                loc: Location::new(0, Span::new(0, 13)),
                left: Some(Box::new(ast::Expression::BooleanLiteral(
                    ast::BooleanLiteral {
                        loc: Location::new(0, Span::new(0, 4)),
                        value: true,
                    },
                ))),
                operator: ast::BinaryOperator::LOr,
                right: Some(Box::new(ast::Expression::BooleanLiteral(
                    ast::BooleanLiteral {
                        loc: Location::new(0, Span::new(8, 13)),
                        value: false,
                    },
                ))),
            }),
            diagnostics: vec![],
        });
    }
}
