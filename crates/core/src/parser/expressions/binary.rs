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

    pub fn parse_binary_expression(&mut self, min_precedence: u8) -> ast::Expression {
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
        let Some((Ok(Token::StarStar), op_range)) = self.tokens.peek().cloned() else {
            return lhs;
        };
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

    #[test]
    fn parse_binary_logical_expression() {
        test_expression(ExpressionTest {
            input: "true || false",
            expected: ast::Expression::Binary(ast::BinaryExpression {
                loc: Location::new(0, Span::new(0, 13)),
                left: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    loc: Location::new(0, Span::new(0, 4)),
                    value: true,
                })),
                operator: ast::BinaryOperator::LOr,
                right: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    loc: Location::new(0, Span::new(8, 13)),
                    value: false,
                })),
            }),
            diagnostics: vec![],
        });
    }
}
