use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::codegen::utils::create_ident;

/// Produce an `Object.assign(this, assigned)` expression
pub fn object_assign_this(assigned: &str) -> swc::Expr {
    swc::Expr::Call(swc::CallExpr {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::Expr::Ident(create_ident("Object"))),
            prop: swc::MemberProp::Ident(create_ident("assign").into()),
        }))),
        args: vec![
            swc::ExprOrSpread {
                spread: None,
                expr: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
            },
            swc::ExprOrSpread {
                spread: None,
                expr: Box::new(swc::Expr::Ident(create_ident(assigned))),
            },
        ],
        type_args: None,
    })
}

// Specifically, create an assignemnent like `this.field_name = field_name;`
pub fn this_assignment(field_name: &str) -> swc::Stmt {
    swc::Stmt::Expr(swc::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
            span: DUMMY_SP,
            op: swc::AssignOp::Assign,
            left: swc::AssignTarget::Simple(swc::SimpleAssignTarget::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
                prop: swc::MemberProp::Ident(create_ident(field_name).into()),
            })),
            right: Box::new(create_ident(field_name).into()),
        })),
    })
}
