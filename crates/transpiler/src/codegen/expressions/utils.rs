use swc_common::{Spanned, SyntaxContext};
use swc_ecma_ast as swc;

pub fn stmt_to_iife(stmt: swc::Stmt) -> swc::Expr {
    swc::Expr::Call(swc::CallExpr {
        span: stmt.span(),
        ctxt: SyntaxContext::empty(),
        args: vec![],
        callee: swc::Callee::Expr(Box::new(swc::Expr::Arrow(swc::ArrowExpr {
            span: stmt.span(),
            ctxt: SyntaxContext::empty(),
            params: vec![],
            body: Box::new(swc::BlockStmtOrExpr::BlockStmt(swc::BlockStmt {
                span: stmt.span(),
                ctxt: SyntaxContext::empty(),
                stmts: vec![stmt],
            })),
            is_async: false,
            is_generator: false,
            type_params: None,
            return_type: None,
        }))),
        type_args: None,
    })
}
