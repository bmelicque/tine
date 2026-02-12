use crate::{
    ast,
    parser::{tokens::Token, Parser},
    DiagnosticKind, Location,
};

impl Parser<'_> {
    pub fn parse_assignment(&mut self) -> Option<ast::Statement> {
        let expr = self.parse_expression();

        let Some((Ok(Token::Eq), eq_range)) = self.tokens.peek() else {
            return expr.map(|e| {
                ast::Statement::Expression(ast::ExpressionStatement {
                    expression: Box::new(e),
                })
            });
        };
        let eq_range = eq_range.clone();
        let eq_loc = self.localize(eq_range);
        self.tokens.next(); // consume the '=' token
        let loc = match &expr {
            Some(expr) => Location::merge(expr.loc(), eq_loc),
            None => eq_loc,
        };
        let assignee = match expr {
            Some(ast::Expression::Member(expr)) => Some(ast::Assignee::Member(expr)),
            Some(ast::Expression::Unary(unary)) if unary.operator == ast::UnaryOperator::Star => {
                match &unary.operand {
                    Some(operand) => match &**operand {
                        ast::Expression::Identifier(ident) => {
                            Some(ast::Assignee::Indirection(ast::IndirectionAssignee {
                                loc: unary.loc,
                                identifier: ident.clone(),
                            }))
                        }
                        expr => Some(self.expr_to_pattern(expr.clone()).into()),
                    },
                    None => None,
                }
            }
            Some(expr) => Some(self.expr_to_pattern(expr).into()),
            None => None,
        };
        if assignee.is_none() {
            self.error(DiagnosticKind::MissingPattern, eq_loc);
        }

        let value = self.parse_expression();
        if value.is_none() {
            self.error(DiagnosticKind::MissingExpression, eq_loc.increment());
        }

        let loc = match &value {
            Some(value) => Location::merge(loc, value.loc()),
            None => loc,
        };

        Some(ast::Statement::Assignment(ast::Assignment {
            loc,
            pattern: assignee,
            value,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        parser::test_utils::{test_statement, StatementTest},
        Diagnostic, DiagnosticLevel, Span,
    };

    use super::*;

    #[test]
    fn test_parse_simple_assignment() {
        test_statement(StatementTest {
            input: "x = 42",
            expected: ast::Statement::Assignment(ast::Assignment {
                loc: Location::new(0, Span::new(0, 6)),
                pattern: Some(ast::Assignee::Pattern(ast::Pattern::Identifier(
                    ast::IdentifierPattern(ast::Identifier {
                        loc: Location::new(0, Span::new(0, 1)),
                        text: "x".into(),
                    }),
                ))),
                value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(4, 6)),
                    value: 42,
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn test_parse_assignment_missing_value() {
        test_statement(StatementTest {
            input: "x =",
            expected: ast::Statement::Assignment(ast::Assignment {
                loc: Location::new(0, Span::new(0, 3)),
                pattern: Some(ast::Assignee::Pattern(ast::Pattern::Identifier(
                    ast::IdentifierPattern(ast::Identifier {
                        loc: Location::new(0, Span::new(0, 1)),
                        text: "x".into(),
                    }),
                ))),
                value: None,
            }),
            diagnostics: vec![Diagnostic {
                kind: DiagnosticKind::MissingExpression,
                loc: Location::new(0, Span::new(3, 4)),
                level: DiagnosticLevel::Error,
            }],
        })
    }

    #[test]
    fn test_parse_member_assignment() {
        test_statement(StatementTest {
            input: "x.y = 42",
            expected: ast::Statement::Assignment(ast::Assignment {
                loc: Location::new(0, Span::new(0, 8)),
                pattern: Some(ast::Assignee::Member(ast::MemberExpression {
                    loc: Location::new(0, Span::new(0, 3)),
                    object: Some(Box::new(ast::Expression::Identifier(ast::Identifier {
                        loc: Location::new(0, Span::new(0, 1)),
                        text: "x".into(),
                    }))),
                    prop: Some(ast::MemberProp::FieldName(ast::Identifier {
                        loc: Location::new(0, Span::new(2, 3)),
                        text: "y".into(),
                    })),
                })),
                value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(6, 8)),
                    value: 42,
                })),
            }),
            diagnostics: vec![],
        });
    }

    #[test]
    fn parse_expression_statement() {
        test_statement(StatementTest {
            input: "42",
            expected: ast::Statement::Expression(ast::ExpressionStatement {
                expression: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(0, 2)),
                    value: 42,
                })),
            }),
            diagnostics: vec![],
        })
    }
}
