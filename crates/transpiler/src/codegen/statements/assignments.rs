use crate::codegen::{utils::create_ident, CodeGenerator};
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;
use tine_core::ast;

impl CodeGenerator<'_> {
    pub fn assignment_to_swc(&mut self, node: &ast::Assignment) -> swc::ExprStmt {
        if let ast::Assignee::Indirection(_) = node.pattern.as_ref().unwrap() {
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
                left: self.assign_target_to_swc(node.pattern.as_ref().unwrap()),
                right: Box::new(self.expr_to_swc(node.value.as_ref().unwrap())),
            })),
        }
    }

    fn assign_target_to_swc(&mut self, node: &ast::Assignee) -> swc::AssignTarget {
        match node {
            ast::Assignee::Member(expr) => {
                swc::SimpleAssignTarget::Member(self.member_expr_to_swc(expr)).into()
            }
            ast::Assignee::Indirection(_) => unreachable!(),

            ast::Assignee::Pattern(pat) => match pat {
                ast::Pattern::Invalid { .. } => unreachable!(),
                ast::Pattern::Identifier(id) => {
                    swc::SimpleAssignTarget::Ident(create_ident(id.as_str()).into()).into()
                }
                ast::Pattern::Literal(_) => unreachable!(),
                ast::Pattern::Tuple(pat) => {
                    swc::AssignTargetPat::Array(self.tuple_pattern_to_swc(pat)).into()
                }
                ast::Pattern::Constructor(pat) => match &pat.constructor {
                    ast::Constructor::Invalid(_) => panic!(),
                    ast::Constructor::Map(_) => unimplemented!(),
                    ast::Constructor::Named(_) | ast::Constructor::Variant(_) => match &pat.body {
                        Some(body) => self.constructor_pattern_body_to_swc(body),
                        None => swc::SimpleAssignTarget::Ident(create_ident("__").into()).into(),
                    },
                },
            },
        }
    }

    fn constructor_pattern_body_to_swc(
        &mut self,
        body: &ast::ConstructorPatternBody,
    ) -> swc::AssignTarget {
        match body {
            ast::ConstructorPatternBody::Struct(st) => {
                self.struct_pattern_to_swc(&st.fields).into()
            }
            ast::ConstructorPatternBody::Tuple(t) => self.tuple_pattern_to_swc(t).into(),
        }
    }

    /**
    Setting a value through a reference, like:
    `*ref = value`

    This is transpiled as:
    ```ref.set(value)```
    */
    fn indirected_assignment_to_swc(&mut self, node: &ast::Assignment) -> swc::Expr {
        let ast::Assignee::Indirection(assignee) = node.pattern.as_ref().unwrap() else {
            panic!("Expected assignment to indirection")
        };

        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(self.ident_to_swc(&assignee.identifier)),
                prop: swc::MemberProp::Ident(create_ident("set").into()),
            }))),
            args: vec![self.expr_to_swc(node.value.as_ref().unwrap()).into()],
            type_args: None,
        })
    }
}
