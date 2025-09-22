use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::{
    ast,
    codegen::{utils::create_ident, CodeGenerator},
};

impl CodeGenerator {
    pub fn assignment_to_swc(&mut self, node: ast::Assignment) -> swc::ExprStmt {
        if let ast::PatternExpression::Expression(ast::Expression::Unary(ast::UnaryExpression {
            operator: ast::UnaryOperator::Star,
            ..
        })) = node.pattern
        {
            return swc::ExprStmt {
                span: DUMMY_SP,
                expr: Box::new(self.indirected_assignment_to_swc(node)),
            };
        }

        swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                span: DUMMY_SP,
                op: swc::AssignOp::Assign,
                left: self.pat_or_expr_to_swc(node.pattern),
                right: Box::new(self.expr_to_swc(node.value)),
            })),
        }
    }

    /**
    Setting a value through a reference, like:
    `*ref = value`

    This is transpiled as:
    ```ref.set(value)```
    */
    fn indirected_assignment_to_swc(&mut self, node: ast::Assignment) -> swc::Expr {
        let ast::PatternExpression::Expression(ast::Expression::Unary(ast::UnaryExpression {
            operator: ast::UnaryOperator::Star,
            operand: assignee,
            ..
        })) = node.pattern
        else {
            panic!("Expected assignment to indirection")
        };

        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(*assignee)),
                prop: swc::MemberProp::Ident(create_ident("set")),
            }))),
            args: vec![self.expr_to_swc(node.value).into()],
            type_args: None,
        })
    }
}
