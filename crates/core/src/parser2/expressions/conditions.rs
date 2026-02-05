use crate::{
    ast,
    parser2::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_condition(&mut self) -> ast::Expression {
        let start_range = self.eat(&[Token::If]);
        let start_loc = self.localize(start_range);
        match self.tokens.peek() {
            Some((Ok(Token::Const | Token::Var), _)) => {
                self.parse_if_pattern_expression(start_loc).into()
            }
            _ => self.parse_if_expression(start_loc).into(),
        }
    }

    fn parse_if_pattern_expression(&mut self, kw: Location) -> ast::IfPatExpression {
        let declaration = self.parse_variable_declaration(None);
        let body = self.parse_if_body();
        let alternate = match self.tokens.peek() {
            Some((Ok(Token::Else), _)) => self.parse_alternate(),
            _ => None,
        };
        let end_loc = match &alternate {
            Some(alternate) => alternate.loc(),
            None => body.loc,
        };
        ast::IfPatExpression {
            loc: Location::merge(kw, end_loc),
            pattern: declaration.pattern,
            scrutinee: declaration.value,
            consequent: Box::new(body),
            alternate: alternate.map(|a| Box::new(a)),
        }
    }

    fn parse_if_expression(&mut self, kw: Location) -> ast::IfExpression {
        let condition = match self.parse_expression_without_block() {
            Some(expr) => expr,
            None => {
                self.report_missing(DiagnosticKind::MissingExpression);
                ast::Expression::Empty
            }
        };
        let consequent = self.parse_if_body();
        println!("{:?}", self.tokens.peek());
        let alternate = match self.tokens.peek() {
            Some((Ok(Token::Else), _)) => self.parse_alternate(),
            _ => None,
        };
        let end_loc = match &alternate {
            Some(alternate) => alternate.loc(),
            None => consequent.loc,
        };
        ast::IfExpression {
            loc: Location::merge(kw, end_loc),
            condition: Box::new(condition),
            consequent: Box::new(consequent),
            alternate: alternate.map(|a| Box::new(a)),
        }
    }

    fn parse_if_body(&mut self) -> ast::BlockExpression {
        if let Some((Ok(Token::LBrace), _)) = self.tokens.peek() {
            self.parse_block()
        } else {
            self.recover_before(&[Token::LBrace], &[Token::Newline]);
            let range = self.next_range();
            ast::BlockExpression {
                statements: vec![],
                loc: self.localize(range),
            }
        }
    }

    fn parse_alternate(&mut self) -> Option<ast::Alternate> {
        self.eat(&[Token::Else]);
        self.expect_either(&[Token::LBrace, Token::If, Token::Newline]);
        match self.tokens.peek() {
            Some((Ok(Token::If), _)) => {
                let alternate = match self.parse_condition() {
                    ast::Expression::If(if_expr) => if_expr.into(),
                    ast::Expression::IfDecl(expr) => expr.into(),
                    ast::Expression::Block(block) => block.into(),
                    _ => unreachable!(),
                };
                Some(alternate)
            }
            Some((Ok(Token::LBrace), _)) => Some(self.parse_block().into()),
            _ => None,
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
    fn test_parse_if_expression() {
        run(Test {
            input: "if true {}",
            expected: ast::Expression::If(ast::IfExpression {
                loc: Location::new(0, Span::new(0, 10)),
                condition: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    loc: Location::new(0, Span::new(3, 7)),
                    value: true,
                })),
                consequent: Box::new(ast::BlockExpression {
                    statements: vec![],
                    loc: Location::new(0, Span::new(8, 10)),
                }),
                alternate: None,
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_if_expression_with_alternate() {
        run(Test {
            input: "if true {} else {}",
            expected: ast::Expression::If(ast::IfExpression {
                loc: Location::new(0, Span::new(0, 18)),
                condition: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    loc: Location::new(0, Span::new(3, 7)),
                    value: true,
                })),
                consequent: Box::new(ast::BlockExpression {
                    statements: vec![],
                    loc: Location::new(0, Span::new(8, 10)),
                }),
                alternate: Some(Box::new(ast::Alternate::Block(ast::BlockExpression {
                    statements: vec![],
                    loc: Location::new(0, Span::new(16, 18)),
                }))),
            }),
            diagnostics: vec![],
        })
    }

    #[test]
    fn test_parse_if_expression_missing_consequent() {
        run(Test {
            input: "if true",
            expected: ast::Expression::If(ast::IfExpression {
                loc: Location::new(0, Span::new(0, 7)),
                condition: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    loc: Location::new(0, Span::new(3, 7)),
                    value: true,
                })),
                consequent: Box::new(ast::BlockExpression {
                    statements: vec![],
                    loc: Location::new(0, Span::new(7, 7)),
                }),
                alternate: None,
            }),
            diagnostics: vec![Diagnostic {
                loc: Location::new(0, Span::new(7, 7)),
                kind: DiagnosticKind::ExpectedToken {
                    expected: vec!["{".to_string()],
                },
                level: DiagnosticLevel::Error,
            }],
        })
    }
}
