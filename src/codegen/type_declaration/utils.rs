use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::codegen::utils::create_ident;

pub fn name_to_swc_param(name: &str) -> swc::ParamOrTsParamProp {
    swc::ParamOrTsParamProp::Param(swc::Param {
        span: DUMMY_SP,
        decorators: vec![],
        pat: swc::Pat::Ident(swc::BindingIdent {
            id: create_ident(name),
            type_ann: None,
        }),
    })
}

// Specifically, create an assignemnent like `this.field_name = field_name;`
pub fn this_assignment(field_name: &str) -> swc::Stmt {
    swc::Stmt::Expr(swc::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
            span: DUMMY_SP,
            op: swc::AssignOp::Assign,
            left: swc::PatOrExpr::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
                prop: create_ident(field_name).into(),
            }))),
            right: Box::new(create_ident(field_name).into()),
        })),
    })
}
