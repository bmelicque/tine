use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_binary_expression(&mut self, min_precedence: u8) -> ast::Expression {
        if min_precedence == Token::StarStar.precedence() {
            return self.parse_exponentiation();
        }
        let mut expression = self.parse_binary_expression(min_precedence + 1);
        while let Some((Ok(token), op_range)) = self.tokens.peek().cloned() {
            if token.precedence() <= min_precedence {
                break;
            }
            self.tokens.next(); // consume the operator
            let operator = token.to_string().into();
            let right = self.parse_binary_expression(min_precedence + 1);
            let loc = if right.is_empty() {
                self.error(
                    DiagnosticKind::MissingExpression,
                    self.localize(op_range.clone()).increment(),
                );
                Location::merge(expression.loc(), self.localize(op_range))
            } else {
                Location::merge(expression.loc(), right.loc())
            };
            expression = ast::Expression::Binary(ast::BinaryExpression {
                loc,
                left: Box::new(expression),
                operator,
                right: Box::new(right),
            })
        }
        expression
    }

    fn parse_exponentiation(&mut self) -> ast::Expression {
        let lhs = self.parse_unary_expression();
        match self.tokens.peek().cloned() {
            Some((Ok(Token::StarStar), op_range)) => {
                self.tokens.next(); // consume the operator
                let rhs = self.parse_exponentiation();
                let loc = if rhs.is_empty() {
                    self.error(
                        DiagnosticKind::MissingExpression,
                        self.localize(op_range.clone()).increment(),
                    );
                    Location::merge(lhs.loc(), self.localize(op_range))
                } else {
                    Location::merge(lhs.loc(), rhs.loc())
                };
                ast::Expression::Binary(ast::BinaryExpression {
                    loc,
                    left: Box::new(lhs),
                    operator: ast::BinaryOperator::Pow,
                    right: Box::new(rhs),
                })
            }
            _ => lhs,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser2::test_utils::{run, Test},
        Span,
    };

    use super::*;

    #[test]
    fn parse_binary_expression() {
        run(Test {
            input: "1 + 2",
            expected: ast::Expression::Binary(ast::BinaryExpression {
                loc: Location::new(0, Span::new(0, 5)),
                left: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(0, 1)),
                    value: 1,
                })),
                operator: ast::BinaryOperator::Add,
                right: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(4, 5)),
                    value: 2,
                })),
            }),
            diagnostics: vec![],
        });
    }
}
