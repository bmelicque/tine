use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use crate::codegen::utils::create_ident;

pub fn name_to_swc_param(name: &str) -> ast::ParamOrTsParamProp {
    ast::ParamOrTsParamProp::Param(ast::Param {
        span: DUMMY_SP,
        decorators: vec![],
        pat: ast::Pat::Ident(ast::BindingIdent {
            id: create_ident(name),
            type_ann: None,
        }),
    })
}

// Specifically, create an assignemnent like `this.field_name = field_name;`
pub fn this_assignment(field_name: &str) -> ast::Stmt {
    ast::Stmt::Expr(ast::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(ast::Expr::Assign(ast::AssignExpr {
            span: DUMMY_SP,
            op: ast::AssignOp::Assign,
            left: ast::PatOrExpr::Expr(Box::new(ast::Expr::Member(ast::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(ast::Expr::This(ast::ThisExpr { span: DUMMY_SP })),
                prop: create_ident(field_name).into(),
            }))),
            right: Box::new(create_ident(field_name).into()),
        })),
    })
}
