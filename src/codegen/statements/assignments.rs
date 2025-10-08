use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::{
    ast,
    codegen::{utils::create_ident, CodeGenerator},
};

impl CodeGenerator {
    pub fn assignment_to_swc(&mut self, node: &ast::Assignment) -> swc::ExprStmt {
        if let ast::Assignee::Indirection(_) = node.pattern {
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
                left: self.assign_target_to_swc(&node.pattern),
                right: Box::new(self.expr_to_swc(&node.value)),
            })),
        }
    }

    fn assign_target_to_swc(&mut self, node: &ast::Assignee) -> swc::AssignTarget {
        match node {
            ast::Assignee::FieldAccess(expr) => {
                swc::SimpleAssignTarget::Member(self.field_access_to_swc(expr)).into()
            }
            ast::Assignee::Indirection(_) => unreachable!(),
            ast::Assignee::TupleIndexing(expr) => {
                swc::SimpleAssignTarget::Member(self.tuple_indexing_to_swc(expr)).into()
            }

            ast::Assignee::Pattern(pat) => match pat {
                ast::Pattern::Identifier(id) => {
                    swc::SimpleAssignTarget::Ident(create_ident(id.span.as_str()).into()).into()
                }
                ast::Pattern::Literal(_) => unreachable!(),
                ast::Pattern::Struct(pat) => {
                    swc::AssignTargetPat::Object(self.struct_pattern_to_swc(&pat.fields)).into()
                }
                ast::Pattern::Tuple(pat) => {
                    swc::AssignTargetPat::Array(self.tuple_pattern_to_swc(pat)).into()
                }
                ast::Pattern::Variant(pat) => match pat.body {
                    Some(ast::VariantPatternBody::Struct(ref fields)) => {
                        self.struct_pattern_to_swc(fields).into()
                    }
                    Some(ast::VariantPatternBody::Tuple(ref body)) => {
                        self.tuple_pattern_to_swc(body).into()
                    }
                    None => swc::SimpleAssignTarget::Ident(create_ident("__").into()).into(),
                },
            },
        }
    }

    /**
    Setting a value through a reference, like:
    `*ref = value`

    This is transpiled as:
    ```ref.set(value)```
    */
    fn indirected_assignment_to_swc(&mut self, node: &ast::Assignment) -> swc::Expr {
        let ast::Assignee::Indirection(assignee) = &node.pattern else {
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
            args: vec![self.expr_to_swc(&node.value).into()],
            type_args: None,
        })
    }
}
