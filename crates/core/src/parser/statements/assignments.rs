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
            Some(ast::Expression::Member(expr)) => ast::Assignee::Member(expr),
            Some(ast::Expression::Unary(unary)) if unary.operator == ast::UnaryOperator::Star => {
                if let ast::Expression::Identifier(ident) = *unary.operand {
                    ast::Assignee::Indirection(ast::IndirectionAssignee {
                        loc: unary.loc,
                        identifier: ident,
                    })
                } else {
                    self.expr_to_pattern(unary.into()).into()
                }
            }
            Some(expr) => self.expr_to_pattern(expr).into(),
            None => {
                self.error(DiagnosticKind::MissingPattern, eq_loc);
                ast::Pattern::Invalid { loc: eq_loc }.into()
            }
        };

        let value = match self.parse_expression() {
            Some(value) => value,
            None => {
                self.error(DiagnosticKind::MissingExpression, eq_loc.increment());
                ast::Expression::Empty
            }
        };
        let loc = match value {
            ast::Expression::Empty => loc,
            _ => Location::merge(loc, value.loc()),
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
                pattern: ast::Assignee::Pattern(ast::Pattern::Identifier(ast::IdentifierPattern(
                    ast::Identifier {
                        loc: Location::new(0, Span::new(0, 1)),
                        text: "x".into(),
                    },
                ))),
                value: ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(4, 6)),
                    value: 42,
                }),
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
                pattern: ast::Assignee::Pattern(ast::Pattern::Identifier(ast::IdentifierPattern(
                    ast::Identifier {
                        loc: Location::new(0, Span::new(0, 1)),
                        text: "x".into(),
                    },
                ))),
                value: ast::Expression::Empty,
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
                pattern: ast::Assignee::Member(ast::MemberExpression {
                    loc: Location::new(0, Span::new(0, 3)),
                    object: Box::new(ast::Expression::Identifier(ast::Identifier {
                        loc: Location::new(0, Span::new(0, 1)),
                        text: "x".into(),
                    })),
                    prop: Some(ast::MemberProp::FieldName(ast::Identifier {
                        loc: Location::new(0, Span::new(2, 3)),
                        text: "y".into(),
                    })),
                }),
                value: ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::new(0, Span::new(6, 8)),
                    value: 42,
                }),
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
