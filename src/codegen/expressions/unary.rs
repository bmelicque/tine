use crate::{
    ast,
    codegen::{
        codegen::TranspilerFlags,
        utils::{create_ident, create_number, create_str},
        CodeGenerator,
    },
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

impl CodeGenerator {
    pub fn unary_expression_to_swc_expr(&mut self, node: ast::UnaryExpression) -> swc::Expr {
        match node.operator {
            ast::UnaryOperator::Deref => self.dereference_to_swc_expr(node),
            ast::UnaryOperator::ImmutableRef | ast::UnaryOperator::MutableRef => {
                self.ref_to_swc_expr(node)
            }
            ast::UnaryOperator::Negate => self.negation_to_swc_expr(node),
            ast::UnaryOperator::Not => self.logical_not_to_swc_expr(node),
        }
    }

    /**
     * `*expr` => `expr.get()`
     */
    fn dereference_to_swc_expr(&mut self, node: ast::UnaryExpression) -> swc::Expr {
        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(*node.operand)),
                prop: swc::MemberProp::Ident(create_ident("get")),
            }))),
            args: vec![],
            type_args: None,
        })
    }

    fn ref_to_swc_expr(&mut self, node: ast::UnaryExpression) -> swc::Expr {
        self.add_flag(TranspilerFlags::Reference);
        let (ctx, value) = match *node.operand {
            ast::Expression::Identifier(expr) => {
                (self.ident_to_swc(expr).into(), create_number(0.0))
            }
            ast::Expression::FieldAccess(expr) => (
                self.expr_to_swc(*expr.object),
                create_str(expr.prop.as_str().into()),
            ),
            ast::Expression::TupleIndexing(expr) => (
                self.expr_to_swc(*expr.tuple),
                create_number(*expr.index.value),
            ),
            expr => (
                swc::Expr::Ident(create_ident("undefined")),
                self.expr_to_swc(expr),
            ),
        };
        swc::Expr::New(swc::NewExpr {
            span: DUMMY_SP,
            callee: Box::new(swc::Expr::Ident(create_ident("__Reference"))),
            args: Some(vec![ctx.into(), value.into()]),
            type_args: None,
        })
    }

    fn negation_to_swc_expr(&mut self, node: ast::UnaryExpression) -> swc::Expr {
        swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Minus,
            arg: Box::new(self.expr_to_swc(*node.operand)),
        })
    }

    fn logical_not_to_swc_expr(&mut self, node: ast::UnaryExpression) -> swc::Expr {
        swc::Expr::Unary(swc::UnaryExpr {
            span: DUMMY_SP,
            op: swc::UnaryOp::Bang,
            arg: Box::new(self.expr_to_swc(*node.operand)),
        })
    }
}
