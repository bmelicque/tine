use crate::{
    ast,
    parser2::{tokens::Token, Parser},
};

impl Parser<'_> {
    pub fn parse_tuple(&mut self) -> ast::TupleExpression {
        let Some((Ok(Token::LParen), start_range)) = self.tokens.next() else {
            panic!("expected (");
        };

        let expressions = self.parse_list(
            |parser| parser.parse_expression(),
            Token::Comma,
            Token::RParen,
        );

        let end_range = match self.tokens.peek() {
            Some((Ok(Token::RParen), r)) => r.clone(),
            _ => self.recover_at(&[Token::RParen]),
        };

        ast::TupleExpression {
            loc: self.localize(start_range.start..end_range.end),
            elements: expressions,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser2::test_utils::{test_expression, ExpressionTest},
        Diagnostic, DiagnosticKind, DiagnosticLevel, Location, Span,
    };

    use super::*;

    #[test]
    fn test_parse_empty_tuple() {
        test_expression(ExpressionTest {
            input: "()",
            expected: ast::Expression::Tuple(ast::TupleExpression {
                loc: Location::new(0, Span::new(0, 2)),
                elements: vec![],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_simple_tuple() {
        test_expression(ExpressionTest {
            input: "(0)",
            expected: ast::Expression::Tuple(ast::TupleExpression {
                loc: Location::new(0, Span::new(0, 3)),
                elements: vec![ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(1, 2)),
                    value: 0,
                })],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_tuple() {
        test_expression(ExpressionTest {
            input: "(0, 1)",
            expected: ast::Expression::Tuple(ast::TupleExpression {
                loc: Location::new(0, Span::new(0, 6)),
                elements: vec![
                    ast::Expression::IntLiteral(ast::IntLiteral {
                        loc: Location::new(0, Span::new(1, 2)),
                        value: 0,
                    }),
                    ast::Expression::IntLiteral(ast::IntLiteral {
                        loc: Location::new(0, Span::new(4, 5)),
                        value: 1,
                    }),
                ],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_tuple_with_trailing_comma() {
        test_expression(ExpressionTest {
            input: "(0,)",
            expected: ast::Expression::Tuple(ast::TupleExpression {
                loc: Location::new(0, Span::new(0, 4)),
                elements: vec![ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(1, 2)),
                    value: 0,
                })],
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_tuple_with_missing_comma() {
        test_expression(ExpressionTest {
            input: "(0 1)",
            expected: ast::Expression::Tuple(ast::TupleExpression {
                loc: Location::new(0, Span::new(0, 5)),
                elements: vec![ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(1, 2)),
                    value: 0,
                })],
            }),
            diagnostics: vec![Diagnostic {
                level: DiagnosticLevel::Error,
                loc: Location::new(0, Span::new(3, 4)),
                kind: DiagnosticKind::ExpectedToken {
                    expected: vec![Token::Newline, Token::Comma, Token::RParen]
                        .into_iter()
                        .map(|d| d.to_string())
                        .collect(),
                },
            }],
        });
    }
}
