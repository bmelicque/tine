use crate::codegen::{
    expressions::ExpressionResult,
    utils::{assign, call, ident_from_str, is_handled_by_ref, member},
    CodeGenerator,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;

impl CodeGenerator<'_, '_> {
    pub fn handle_assignment(&mut self, node: ir::Assignment) -> Vec<swc::Stmt> {
        let result = self.handle_assignment_as_expr(node);
        let mut stmts = result.prelim_stmts;
        stmts.push(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(result.expr),
        }));
        stmts
    }

    pub fn handle_assignment_as_expr(&mut self, node: ir::Assignment) -> ExpressionResult {
        match node.pattern {
            ir::Expression::Identifier(p) => {
                self.handle_raw_assign(p.into(), node.operator, node.value)
            }
            ir::Expression::Member(p) => {
                self.handle_raw_assign(p.into(), node.operator, node.value)
            }
            ir::Expression::Unary(u) if u.operator == ir::UnaryOperator::Star => {
                self.handle_indirect_assign(*u.operand, node.operator, node.value)
            }
            _ => unimplemented!(),
        }
    }

    fn handle_raw_assign(
        &mut self,
        assign_target: ir::Expression,
        operator: ir::AssignOperator,
        value: ir::Expression,
    ) -> ExpressionResult {
        let is_current_this = self.is_current_this(&assign_target);

        let value_result = self.handle_assigned_value(value);

        let assign_target = if value_result.prelim_stmts.len() > 0 {
            let result = self.handle_expression(assign_target);
            self.to_extracted(result)
        } else {
            self.handle_expression(assign_target)
        };

        let prelim_stmts = vec![assign_target.prelim_stmts, value_result.prelim_stmts].concat();

        let expr = if is_current_this {
            // Use `Object.assign(this, ...)` to mutate the receiver on raw `this = ...`
            swc::Expr::Call(swc::CallExpr {
                callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                    obj: Box::new(ident_from_str("Object").into()),
                    prop: swc::MemberProp::Ident(ident_from_str("assign").into()),
                    ..Default::default()
                }))),
                args: vec![assign_target.expr.into(), value_result.expr.into()],
                ..Default::default()
            })
        } else {
            let assign_target = match assign_target.expr {
                swc::Expr::Ident(i) => swc::SimpleAssignTarget::Ident(i.into()),
                swc::Expr::Member(m) => swc::SimpleAssignTarget::Member(m),
                _ => unreachable!(),
            };
            swc::Expr::Assign(swc::AssignExpr {
                op: handle_assign_op(operator),
                left: assign_target.into(),
                right: Box::new(value_result.expr),
                ..Default::default()
            })
        };

        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_indirect_assign(
        &mut self,
        assignee: ir::Expression,
        operator: ir::AssignOperator,
        value: ir::Expression,
    ) -> ExpressionResult {
        let value_result = self.handle_expression(value);

        let assign_target = if value_result.prelim_stmts.len() > 0 {
            let result = self.handle_expression(assignee);
            self.to_extracted(result)
        } else {
            self.handle_expression(assignee)
        };

        let prelim_stmts = vec![assign_target.prelim_stmts, value_result.prelim_stmts].concat();

        // `signal.setupTreeUpdate().value <OP> <VALUE>`
        let expr = swc::Expr::Assign(assign(
            member(
                call(member(assign_target.expr, "setupTreeUpdate").into(), vec![]).into(),
                "value",
            )
            .into(),
            handle_assign_op(operator),
            value_result.expr,
        ));

        ExpressionResult { prelim_stmts, expr }
    }

    pub(crate) fn handle_assigned_value(&mut self, value: ir::Expression) -> ExpressionResult {
        let is_ref = is_handled_by_ref(&value);
        let mut result = self.handle_expression(value);

        if is_ref {
            result.expr = swc::Expr::Call(swc::CallExpr {
                callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                    span: DUMMY_SP,
                    obj: Box::new(result.expr),
                    prop: swc::MemberProp::Ident(ident_from_str("$get").into()),
                }))),
                ..Default::default()
            })
        }
        result
    }
}

fn handle_assign_op(op: ir::AssignOperator) -> swc::AssignOp {
    match op {
        ir::AssignOperator::Assign => swc::AssignOp::Assign,
        ir::AssignOperator::AddAssign => swc::AssignOp::AddAssign,
        ir::AssignOperator::SubAssign => swc::AssignOp::SubAssign,
        ir::AssignOperator::MulAssign => swc::AssignOp::MulAssign,
        ir::AssignOperator::DivAssign => swc::AssignOp::DivAssign,
        ir::AssignOperator::ModAssign => swc::AssignOp::ModAssign,
    }
}

pub(crate) fn assignment(lhs: swc::Expr, rhs: swc::Expr) -> swc::Stmt {
    let lhs = match lhs {
        swc::Expr::Ident(i) => i.into(),
        swc::Expr::Member(m) => m.into(),
        _ => panic!(),
    };

    swc::Stmt::Expr(swc::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
            left: swc::AssignTarget::Simple(lhs),
            right: Box::new(rhs),
            ..Default::default()
        })),
    })
}
