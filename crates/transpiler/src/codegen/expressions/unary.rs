use crate::codegen::{expressions::ExpressionResult, utils::ident_from_str, CodeGenerator};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn handle_unary_expression(&mut self, node: &ir::UnaryExpression) -> ExpressionResult {
        match node.operator {
            ir::UnaryOperator::Bang => self.handle_logical_not(node),
            ir::UnaryOperator::Minus => self.handle_negation(node),
            ir::UnaryOperator::Star => self.handle_deref(node),
            ir::UnaryOperator::Mut => unreachable!(),
        }
    }

    /**
     * `*expr` => `expr.$get()`
     */
    fn handle_deref(&mut self, node: &ir::UnaryExpression) -> ExpressionResult {
        let obj_result = self.handle_expression(&node.operand);

        let expr = swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(obj_result.expr),
                prop: swc::MemberProp::Ident(ident_from_str("$get").into()),
            }))),
            args: vec![],
            type_args: None,
        });

        ExpressionResult {
            prelim_stmts: obj_result.prelim_stmts,
            expr,
        }
    }

    fn handle_negation(&mut self, node: &ir::UnaryExpression) -> ExpressionResult {
        let arg_result = self.handle_expression(&node.operand);

        let expr = swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Minus,
            arg: Box::new(arg_result.expr),
        });

        ExpressionResult {
            prelim_stmts: arg_result.prelim_stmts,
            expr,
        }
    }

    fn handle_logical_not(&mut self, node: &ir::UnaryExpression) -> ExpressionResult {
        let arg_result = self.handle_expression(&node.operand);

        let expr = swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Bang,
            arg: Box::new(arg_result.expr),
        });

        ExpressionResult {
            prelim_stmts: arg_result.prelim_stmts,
            expr,
        }
    }
}
