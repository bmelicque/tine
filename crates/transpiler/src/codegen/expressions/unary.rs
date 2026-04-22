use crate::codegen::{
    expressions::ExpressionResult,
    utils::{ident_from_str, create_str, is_primitive, make_cell},
    CodeGenerator,
};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn handle_unary_expression(&mut self, node: &ir::UnaryExpression) -> ExpressionResult {
        match node.operator {
            ir::UnaryOperator::Ampersand => self.handle_ref(node),
            ir::UnaryOperator::Bang => self.handle_logical_not(node),
            ir::UnaryOperator::Minus => self.handle_negation(node),
            ir::UnaryOperator::Star => self.handle_deref(node),
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

    fn handle_ref(&mut self, node: &ir::UnaryExpression) -> ExpressionResult {
        if !is_primitive(node.operand.ty()) {
            return self.handle_expression(&node.operand);
        }

        match &*node.operand {
            ir::Expression::Identifier(i) => self.handle_identifier(i).into(),
            ir::Expression::Member(m) => self.handle_primitive_member_ref(m),
            _ => {
                let operand = self.handle_expression(&node.operand);
                ExpressionResult {
                    prelim_stmts: operand.prelim_stmts,
                    expr: make_cell(operand.expr),
                }
            }
        }
    }

    /// Handle expressions like `&object.value` that evaluate to a primitive
    fn handle_primitive_member_ref(&mut self, node: &ir::MemberExpression) -> ExpressionResult {
        // `$.MemberRef`
        let callee = swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(ident_from_str("$").into()),
            prop: swc::MemberProp::Ident(ident_from_str("MemberRef").into()),
        });
        let obj = self.handle_expression(&node.object);
        let prop = create_str(&node.member.as_name());

        let expr = swc::Expr::New(swc::NewExpr {
            callee: Box::new(callee),
            args: Some(vec![obj.expr.into(), prop.into()]),
            ..Default::default()
        });

        ExpressionResult {
            prelim_stmts: obj.prelim_stmts,
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
