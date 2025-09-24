use swc_common::{SyntaxContext, DUMMY_SP};
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
                left: self.assign_target_to_swc(node.pattern),
                right: Box::new(self.expr_to_swc(node.value)),
            })),
        }
    }

    fn assign_target_to_swc(&mut self, node: ast::PatternExpression) -> swc::AssignTarget {
        match node {
            ast::PatternExpression::Expression(e) => match e {
                ast::Expression::Array(_) => todo!(),
                ast::Expression::FieldAccess(f) => {
                    swc::SimpleAssignTarget::Member(self.field_access_to_swc(f)).into()
                }
                ast::Expression::Identifier(i) => self.ident_to_swc_assign_target(i).into(),
                ast::Expression::Tuple(_) => todo!(),
                ast::Expression::TupleIndexing(t) => {
                    swc::SimpleAssignTarget::Member(self.tuple_indexing_to_swc(t)).into()
                }
                _ => unreachable!(),
            },
            ast::PatternExpression::Pattern(p) => match p {
                ast::Pattern::Literal(_) => todo!(),
                ast::Pattern::Identifier(_) => swc::SimpleAssignTarget::Ident(swc::BindingIdent {
                    id: todo!(),
                    type_ann: None,
                })
                .into(),
                ast::Pattern::Struct(s) => {
                    swc::AssignTargetPat::Object(self.struct_pattern_to_swc(s.fields)).into()
                }
                _ => todo!(),
            },
        }
    }

    fn ident_to_swc_assign_target(&mut self, ident: ast::Identifier) -> swc::SimpleAssignTarget {
        let ident = self.ident_to_swc(ident);
        match ident {
            swc::Expr::Ident(i) => swc::SimpleAssignTarget::Ident(i.into()),
            _ => unreachable!(),
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
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.expr_to_swc(*assignee)),
                prop: swc::MemberProp::Ident(create_ident("set").into()),
            }))),
            args: vec![self.expr_to_swc(node.value).into()],
            type_args: None,
        })
    }
}
