use crate::codegen::{
    expressions::ExpressionResult,
    utils::{ident_from_str, is_handled_by_ref},
    CodeGenerator,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn handle_assignment(&mut self, node: &ir::Assignment) -> Vec<swc::Stmt> {
        let result = self.handle_assignment_as_expr(node);
        let mut stmts = result.prelim_stmts;
        stmts.push(swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(result.expr),
        }));
        stmts
    }

    pub fn handle_assignment_as_expr(&mut self, node: &ir::Assignment) -> ExpressionResult {
        match &node.pattern {
            ir::Expression::Identifier(_) if is_handled_by_ref(&node.pattern) => {
                self.handle_method_assign(&node.pattern, &node.value)
            }
            ir::Expression::Identifier(_) => self.handle_raw_assign(&node.pattern, &node.value),
            ir::Expression::Member(_) => self.handle_raw_assign(&node.pattern, &node.value),
            ir::Expression::Unary(u) if u.operator == ir::UnaryOperator::Star => {
                self.handle_method_assign(&u.operand, &node.value)
            }
            _ => unimplemented!(),
        }
    }

    fn handle_raw_assign(
        &mut self,
        assign_target: &ir::Expression,
        value: &ir::Expression,
    ) -> ExpressionResult {
        let value_result = self.handle_assigned_value(value);

        let assign_target = if value_result.prelim_stmts.len() > 0 {
            let result = self.handle_expression(assign_target);
            self.to_extracted(result)
        } else {
            self.handle_expression(assign_target)
        };

        let prelim_stmts = vec![assign_target.prelim_stmts, value_result.prelim_stmts].concat();
        let assign_target = match assign_target.expr {
            swc::Expr::Ident(i) => swc::SimpleAssignTarget::Ident(i.into()),
            swc::Expr::Member(m) => swc::SimpleAssignTarget::Member(m),
            _ => unreachable!(),
        };

        let expr = swc::Expr::Assign(swc::AssignExpr {
            left: assign_target.into(),
            right: Box::new(value_result.expr),
            ..Default::default()
        });

        ExpressionResult { prelim_stmts, expr }
    }

    fn handle_method_assign(
        &mut self,
        assign_target: &ir::Expression,
        value: &ir::Expression,
    ) -> ExpressionResult {
        let method_name = if is_handled_by_ref(value) {
            ident_from_str("$assign")
        } else {
            ident_from_str("$set")
        };
        let value_result = self.handle_expression(value);

        let assign_target = if value_result.prelim_stmts.len() > 0 {
            let result = self.handle_expression(assign_target);
            self.to_extracted(result)
        } else {
            self.handle_expression(assign_target)
        };

        let prelim_stmts = vec![assign_target.prelim_stmts, value_result.prelim_stmts].concat();

        let expr = swc::Expr::Call(swc::CallExpr {
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(assign_target.expr),
                prop: swc::MemberProp::Ident(method_name.into()),
            }))),
            args: vec![value_result.expr.into()],
            ..Default::default()
        });

        ExpressionResult { prelim_stmts, expr }
    }

    pub(crate) fn handle_assigned_value(&mut self, value: &ir::Expression) -> ExpressionResult {
        let mut result = self.handle_expression(value);

        if is_handled_by_ref(value) {
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
