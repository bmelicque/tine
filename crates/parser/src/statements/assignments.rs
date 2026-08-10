use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_assignment(&mut self) -> Option<Statement> {
        let expr = self.parse_expression();

        let Some((Ok(Token::Eq), eq_range)) = self.tokens.peek() else {
            return expr.map(|e| {
                Statement::Expression(ExpressionStatement {
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
        let assignee = self.expr_to_assignee(expr);
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

        Some(Statement::Assignment(Assignment {
            loc,
            pattern: assignee,
            value,
        }))
    }

    fn expr_to_assignee(&mut self, expr: Option<Expression>) -> Option<Assignee> {
        match expr? {
            Expression::Path(expr) => Some(Assignee::Member(expr)),
            Expression::Unary(unary) if unary.operator == UnaryOperator::Star => {
                match unary.operand.as_deref()? {
                    Expression::Identifier(ident) => {
                        Some(Assignee::Indirection(IndirectionAssignee {
                            loc: unary.loc,
                            identifier: ident.clone(),
                        }))
                    }
                    expr => Some(self.expr_to_pattern(expr.clone()).into()),
                }
            }
            expr => Some(self.expr_to_pattern(expr).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use tine_common::locations::Span;

    use crate::test_utils::{parse_statement, test_statement, StatementTest};

    use super::*;

    #[test]
    fn parse_simple_assignment() {
        let stmt = parse_statement("x = 42").expect("expected no errors");
        let stmt = stmt.as_assignment().expect("expected an assignment");
        stmt.pattern.as_ref().expect("expected a valid assignee");
        stmt.value.as_ref().expect("expected a valid value");
    }

    #[test]
    fn parse_assignment_missing_value() {
        let (stmt, diags) = parse_statement("x =").expect_err("expected errors");
        let stmt = stmt.expect("expected an assignment");
        let stmt = stmt.as_assignment().expect("expected an assignment");
        stmt.pattern.as_ref().expect("expected a valid assignee");
        assert!(stmt.value.is_none(), "expected no value");

        assert_eq!(diags.len(), 1);
        assert!(matches!(diags[0].kind, DiagnosticKind::MissingExpression))
    }

    #[test]
    fn parse_member_assignment() {
        let stmt = parse_statement("x.y = 42").expect("expected no errors");
        let stmt = stmt.as_assignment().expect("expected an assignment");
        let pattern = stmt.pattern.as_ref().expect("expected a valid assignee");
        pattern.as_member().expect("expected member assignee");
    }

    #[test]
    fn parse_expression_statement() {
        test_statement(StatementTest {
            input: "42",
            expected: Statement::Expression(ExpressionStatement {
                expression: Box::new(Expression::IntLiteral(IntLiteral {
                    loc: Location::new(0, Span::new(0, 2)),
                    value: 42,
                })),
            }),
            diagnostics: vec![],
        })
    }
}
