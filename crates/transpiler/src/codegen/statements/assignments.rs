use crate::codegen::{utils::create_ident, CodeGenerator};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn assignment_to_swc(&mut self, node: &ir::Assignment) -> swc::ExprStmt {
        match &node.pattern {
            ir::Expression::Unary(u) if u.operator == ir::UnaryOperator::Star => {
                return swc::ExprStmt {
                    span: DUMMY_SP,
                    expr: Box::new(self.indirected_assignment_to_swc(node)),
                };
            }
            _ => {}
        };

        swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(self.assignment_to_swc_expr(node).into()),
        }
    }

    pub fn assignment_to_swc_expr(&mut self, node: &ir::Assignment) -> swc::AssignExpr {
        swc::AssignExpr {
            span: DUMMY_SP,
            op: swc::AssignOp::Assign,
            left: self.assign_target_to_swc(&node.pattern),
            right: Box::new(self.expr_to_swc(&node.value)),
        }
    }

    pub fn assign_target_to_swc(&mut self, node: &ir::Expression) -> swc::AssignTarget {
        return match node {
            ir::Expression::Identifier(i) => {
                swc::SimpleAssignTarget::Ident(create_ident(&i.as_name()).into()).into()
            }
            ir::Expression::Member(expr) => {
                swc::SimpleAssignTarget::Member(self.member_expr_to_swc(expr)).into()
            }
            _ => panic!(),
        };
    }

    /**
    Setting a value through a reference, like:
    `*ref = value`

    This is transpiled as:
    ```ref.set(value)```
    */
    fn indirected_assignment_to_swc(&mut self, node: &ir::Assignment) -> swc::Expr {
        let ir::Expression::Unary(assignee) = &node.pattern else {
            panic!("Expected assignment to indirection")
        };

        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(&*assignee.operand)),
                prop: swc::MemberProp::Ident(create_ident("set").into()),
            }))),
            args: vec![self.expr_to_swc(&node.value).into()],
            type_args: None,
        })
    }
}
