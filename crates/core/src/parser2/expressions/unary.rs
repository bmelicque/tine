use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    const UNARY_OPERATORS: [Token; 6] = [
        Token::And,
        Token::At,
        Token::Bang,
        Token::Dollar,
        Token::Minus,
        Token::Star,
    ];

    pub fn parse_unary_expression(&mut self) -> ast::Expression {
        match self.tokens.peek().cloned() {
            Some((Ok(token), op_range)) if Self::UNARY_OPERATORS.contains(&token) => {
                self.tokens.next(); // consume the operator
                let expr = Box::new(self.parse_unary_expression());
                if expr.is_empty() {
                    self.error(
                        DiagnosticKind::MissingExpression,
                        self.localize(op_range.clone()).increment(),
                    );
                }
                ast::Expression::Unary(ast::UnaryExpression {
                    loc: Location::merge(self.localize(op_range), expr.loc()),
                    operator: token.to_string().into(),
                    operand: expr,
                })
            }
            _ => self.parse_postfix(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser2::test_utils::{run, Test},
        Diagnostic, DiagnosticLevel, Span,
    };

    use super::*;

    #[test]
    fn test_parse_unary() {
        run(Test {
            input: "&a",
            expected: ast::Expression::Unary(ast::UnaryExpression {
                loc: Location::new(0, Span::new(0, 2)),
                operator: ast::UnaryOperator::Ampersand,
                operand: Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(1, 2)),
                    text: "a".into(),
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_nested_unary() {
        run(Test {
            input: "&*a",
            expected: ast::Expression::Unary(ast::UnaryExpression {
                loc: Location::new(0, Span::new(0, 3)),
                operator: ast::UnaryOperator::Ampersand,
                operand: Box::new(ast::Expression::Unary(ast::UnaryExpression {
                    loc: Location::new(0, Span::new(1, 3)),
                    operator: ast::UnaryOperator::Star,
                    operand: Box::new(ast::Expression::Identifier(ast::Identifier {
                        loc: Location::new(0, Span::new(2, 3)),
                        text: "a".into(),
                    })),
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_unary_with_missing_expression() {
        run(Test {
            input: "&",
            expected: ast::Expression::Unary(ast::UnaryExpression {
                loc: Location::new(0, Span::new(0, 1)),
                operator: ast::UnaryOperator::Ampersand,
                operand: Box::new(ast::Expression::Empty),
            }),
            diagnostics: vec![Diagnostic {
                kind: DiagnosticKind::MissingExpression,
                loc: Location::new(0, Span::new(1, 2)),
                level: DiagnosticLevel::Error,
            }],
        });
    }
}
