use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

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
                prop: ast::MemberProp::Ident(ast::Ident {
                    span: DUMMY_SP,
                    sym: field_name.into(),
                    optional: false,
                }),
            }))),
            right: Box::new(ast::Expr::Ident(ast::Ident {
                span: DUMMY_SP,
                sym: field_name.into(),
                optional: false,
            })),
        })),
    })
}
