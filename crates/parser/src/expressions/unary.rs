use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};

use crate::{tokens::Token, Parser};

const UNARY_OPERATORS: [Token; 3] = [Token::Bang, Token::Minus, Token::Star];
const EXTENDED_UNARY_OPERATORS: [Token; 4] = [Token::Bang, Token::Minus, Token::Mut, Token::Star];

impl Parser<'_> {
    pub fn parse_unary_expression(&mut self) -> Option<ast::Expression> {
        let operators: &[Token] = if self.in_mutable_binding {
            &EXTENDED_UNARY_OPERATORS
        } else {
            &UNARY_OPERATORS
        };
        match self.tokens.peek().cloned() {
            Some((Ok(token), op_range)) if operators.contains(&token) => {
                self.tokens.next(); // consume the operator
                let expr = self.parse_unary_expression();
                if expr.is_none() {
                    self.error(
                        DiagnosticKind::MissingExpression,
                        self.localize(op_range.clone()).increment(),
                    );
                }
                let loc = match &expr {
                    Some(expr) => Location::merge(self.localize(op_range), expr.loc()),
                    None => self.localize(op_range),
                };
                Some(ast::Expression::Unary(ast::UnaryExpression {
                    loc,
                    operator: token.to_string().into(),
                    operand: expr.map(|e| Box::new(e)),
                }))
            }
            _ => self.parse_postfix(),
        }
    }
}

#[cfg(test)]
mod tests {
    use tine_common::{
        diagnostics::{Diagnostic, DiagnosticLevel},
        locations::Span,
    };

    use crate::test_utils::{test_expression, ExpressionTest};

    use super::*;

    #[test]
    fn test_parse_unary() {
        test_expression(ExpressionTest {
            input: "*a",
            expected: ast::Expression::Unary(ast::UnaryExpression {
                loc: Location::new(0, Span::new(0, 2)),
                operator: ast::UnaryOperator::Star,
                operand: Some(Box::new(ast::Expression::Identifier(ast::Identifier {
                    loc: Location::new(0, Span::new(1, 2)),
                    text: "a".into(),
                }))),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_nested_unary() {
        test_expression(ExpressionTest {
            input: "!*a",
            expected: ast::Expression::Unary(ast::UnaryExpression {
                loc: Location::new(0, Span::new(0, 3)),
                operator: ast::UnaryOperator::Bang,
                operand: Some(Box::new(ast::Expression::Unary(ast::UnaryExpression {
                    loc: Location::new(0, Span::new(1, 3)),
                    operator: ast::UnaryOperator::Star,
                    operand: Some(Box::new(ast::Expression::Identifier(ast::Identifier {
                        loc: Location::new(0, Span::new(2, 3)),
                        text: "a".into(),
                    }))),
                }))),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_unary_with_missing_expression() {
        test_expression(ExpressionTest {
            input: "!",
            expected: ast::Expression::Unary(ast::UnaryExpression {
                loc: Location::new(0, Span::new(0, 1)),
                operator: ast::UnaryOperator::Bang,
                operand: None,
            }),
            diagnostics: vec![Diagnostic {
                kind: DiagnosticKind::MissingExpression,
                loc: Location::new(0, Span::new(1, 2)),
                level: DiagnosticLevel::Error,
            }],
        });
    }
}
