use crate::codegen::{
    utils::{create_ident, create_number, create_str, undefined},
    CodeGenerator,
};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn unary_expression_to_swc_expr(&mut self, node: &ir::UnaryExpression) -> swc::Expr {
        match node.operator {
            ir::UnaryOperator::Ampersand => self.ref_to_swc_expr(node),
            ir::UnaryOperator::Bang => self.logical_not_to_swc_expr(node),
            ir::UnaryOperator::Minus => self.negation_to_swc_expr(node),
            ir::UnaryOperator::Star => self.indirection_to_swc_expr(node),
        }
    }

    /**
     * `*expr` => `expr.get()`
     */
    fn indirection_to_swc_expr(&mut self, node: &ir::UnaryExpression) -> swc::Expr {
        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(&node.operand)),
                prop: swc::MemberProp::Ident(create_ident("get").into()),
            }))),
            args: vec![],
            type_args: None,
        })
    }

    fn ref_to_swc_expr(&mut self, node: &ir::UnaryExpression) -> swc::Expr {
        let (ctx, value) = match &*node.operand {
            ir::Expression::Identifier(expr) => {
                (self.ident_to_swc(expr).into(), create_number(0.0))
            }
            ir::Expression::Member(expr) => (
                self.expr_to_swc(&expr.object),
                match expr.member.as_name().parse::<usize>() {
                    Ok(int) => create_number(int as f64),
                    Err(_) => create_str(&expr.member.as_name()),
                },
            ),
            expr => (undefined(), self.expr_to_swc(expr)),
        };
        swc::Expr::New(swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Ident(create_ident("__Reference"))),
            args: Some(vec![ctx.into(), value.into()]),
            type_args: None,
        })
    }

    fn negation_to_swc_expr(&mut self, node: &ir::UnaryExpression) -> swc::Expr {
        swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Minus,
            arg: Box::new(self.expr_to_swc(&node.operand)),
        })
    }

    fn logical_not_to_swc_expr(&mut self, node: &ir::UnaryExpression) -> swc::Expr {
        swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Bang,
            arg: Box::new(self.expr_to_swc(&node.operand)),
        })
    }
}
