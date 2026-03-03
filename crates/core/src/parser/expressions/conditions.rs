use crate::{
    ast,
    parser::{tokens::Token, Parser},
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
        let end_loc = match (&alternate, &body) {
            (Some(alternate), _) => alternate.loc(),
            (None, Some(consequent)) => consequent.loc,
            _ => declaration.loc,
        };
        ast::IfPatExpression {
            loc: Location::merge(kw, end_loc),
            pattern: declaration.pattern,
            scrutinee: declaration.value.map(|v| Box::new(v)),
            consequent: body,
            alternate: alternate.map(|a| Box::new(a)),
        }
    }

    fn parse_if_expression(&mut self, kw: Location) -> ast::IfExpression {
        let condition = self.parse_expression_without_block();
        if condition.is_none() {
            self.report_missing(DiagnosticKind::MissingExpression);
        }
        let consequent = self.parse_if_body();
        let alternate = match self.tokens.peek() {
            Some((Ok(Token::Else), _)) => self.parse_alternate(),
            _ => None,
        };
        let end_loc = if let Some(alt) = &alternate {
            alt.loc()
        } else if let Some(consequent) = &consequent {
            consequent.loc
        } else if let Some(condition) = &condition {
            condition.loc()
        } else {
            kw
        };

        ast::IfExpression {
            loc: Location::merge(kw, end_loc),
            condition: condition.map(|c| Box::new(c)),
            consequent,
            alternate: alternate.map(|a| Box::new(a)),
        }
    }

    fn parse_if_body(&mut self) -> Option<ast::BlockExpression> {
        if let Some((Ok(Token::LBrace), _)) = self.tokens.peek() {
            Some(self.parse_block())
        } else {
            let error_loc = self.next_loc();
            self.error(DiagnosticKind::MissingConsequent, error_loc);
            None
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
        parser::test_utils::{test_expression, ExpressionTest},
        Diagnostic, DiagnosticLevel, Span,
    };

    use super::*;

    #[test]
    fn test_parse_if_expression() {
        test_expression(ExpressionTest {
            input: "if true {}",
            expected: ast::Expression::If(ast::IfExpression {
                loc: Location::new(0, Span::new(0, 10)),
                condition: Some(Box::new(ast::Expression::BooleanLiteral(
                    ast::BooleanLiteral {
                        loc: Location::new(0, Span::new(3, 7)),
                        value: true,
                    },
                ))),
                consequent: Some(ast::BlockExpression {
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
        test_expression(ExpressionTest {
            input: "if true {} else {}",
            expected: ast::Expression::If(ast::IfExpression {
                loc: Location::new(0, Span::new(0, 18)),
                condition: Some(Box::new(ast::Expression::BooleanLiteral(
                    ast::BooleanLiteral {
                        loc: Location::new(0, Span::new(3, 7)),
                        value: true,
                    },
                ))),
                consequent: Some(ast::BlockExpression {
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
        test_expression(ExpressionTest {
            input: "if true",
            expected: ast::Expression::If(ast::IfExpression {
                loc: Location::new(0, Span::new(0, 7)),
                condition: Some(Box::new(ast::Expression::BooleanLiteral(
                    ast::BooleanLiteral {
                        loc: Location::new(0, Span::new(3, 7)),
                        value: true,
                    },
                ))),
                consequent: None,
                alternate: None,
            }),
            diagnostics: vec![Diagnostic {
                loc: Location::new(0, Span::new(7, 7)),
                kind: DiagnosticKind::MissingConsequent,
                level: DiagnosticLevel::Error,
            }],
        })
    }
}
