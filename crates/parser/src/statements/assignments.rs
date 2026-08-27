use tine_ast::*;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_assignment(&mut self) -> Option<Statement> {
        let expr = self.parse_expression_with_block();

        let Some((operator, op_loc)) = self.maybe_parse_assign_operator() else {
            return expr.map(|e| {
                Statement::Expression(ExpressionStatement {
                    expression: Box::new(e),
                })
            });
        };

        let assignee = expr.map(|e| self.expr_to_assignee(e));
        if assignee.is_none() {
            self.error(DiagnosticKind::MissingPattern, op_loc);
        }

        let value = self.parse_expression_with_block();
        if value.is_none() {
            self.error(DiagnosticKind::MissingExpression, op_loc.increment());
        }

        let loc = assignment_loc(&assignee, op_loc, &value);

        Some(Statement::Assignment(Assignment {
            loc,
            pattern: assignee,
            operator,
            value,
        }))
    }

    fn maybe_parse_assign_operator(&mut self) -> Option<(AssignOperator, Location)> {
        self.maybe_eat(|t| match *t {
            Token::Eq => Some(AssignOperator::Assign),
            Token::PlusEq => Some(AssignOperator::AddAssign),
            Token::MinusEq => Some(AssignOperator::SubAssign),
            Token::StarEq => Some(AssignOperator::MulAssign),
            Token::SlashEq => Some(AssignOperator::DivAssign),
            Token::ModEq => Some(AssignOperator::ModAssign),
            _ => None,
        })
    }

    fn expr_to_assignee(&mut self, expr: Expression) -> Assignee {
        match expr {
            Expression::Path(expr) => Assignee::Path(expr),
            Expression::Unary(expr) => self.unary_expr_to_assignee(expr),
            Expression::Struct(expr) => self.struct_expr_to_assignee(expr),
            Expression::Tuple(expr) => self.tuple_expr_to_assignee(expr),
            e => Assignee::Invalid(InvalidExpression { loc: e.loc() }),
        }
    }

    fn unary_expr_to_assignee(&mut self, expr: UnaryExpression) -> Assignee {
        match expr.operator {
            UnaryOperator::Star => Assignee::Indirection(IndirectionAssignee {
                loc: expr.loc,
                inner: expr
                    .operand
                    .map(|e| self.expr_to_assignee(*e))
                    .map(Box::new),
            }),
            _ => Assignee::Invalid(InvalidExpression { loc: expr.loc }),
        }
    }
    fn struct_expr_to_assignee(&mut self, expr: StructExpression) -> Assignee {
        Assignee::Struct(StructAssignee {
            loc: expr.loc,
            constructor: expr.constructor,
            fields: expr
                .fields
                .into_iter()
                .map(|f| self.struct_expr_field_to_assignee_field(f))
                .collect(),
        })
    }
    fn struct_expr_field_to_assignee_field(
        &mut self,
        field: StructExprField,
    ) -> StructAssigneeField {
        let key = field.key.map(|k| match k {
            StructExprFieldKey::MapKey(k) => {
                StructAssigneeFieldKey::Invalid(InvalidExpression { loc: k.loc() })
            }
            StructExprFieldKey::Name(i) => StructAssigneeFieldKey::Identifier(i),
        });
        StructAssigneeField {
            loc: field.loc,
            key,
            value: field.value.map(|v| self.expr_to_assignee(v)),
        }
    }

    fn tuple_expr_to_assignee(&mut self, expr: TupleExpression) -> Assignee {
        let elements = expr
            .elements
            .into_iter()
            .map(|e| self.expr_to_assignee(e))
            .collect();
        Assignee::Tuple(TupleAssignee {
            loc: expr.loc,
            elements,
        })
    }
}

fn assignment_loc(
    assignee: &Option<Assignee>,
    op_loc: Location,
    value: &Option<Expression>,
) -> Location {
    let start = assignee.as_ref().map_or(op_loc, |a| a.loc());
    let end = value.as_ref().map_or(op_loc, |v| v.loc());
    Location::merge(start, end)
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
    fn parse_indirect_assignment() {
        let stmt = parse_statement("*y = 42").expect("expected no errors");
        let stmt = stmt.as_assignment().expect("expected an assignment");
        let pattern = stmt.pattern.as_ref().expect("expected a valid assignee");
        pattern
            .as_indirection()
            .expect("expected indirection assignee");
    }

    #[test]
    fn parse_member_assignment() {
        let stmt = parse_statement("x.y = 42").expect("expected no errors");
        let stmt = stmt.as_assignment().expect("expected an assignment");
        let pattern = stmt.pattern.as_ref().expect("expected a valid assignee");
        pattern.as_path().expect("expected path assignee");
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
