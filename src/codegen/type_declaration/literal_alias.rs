use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::utils::{name_to_swc_param, this_assignment};

pub fn literal_alias_to_swc_constructor() -> ast::Constructor {
    ast::Constructor {
        span: DUMMY_SP,
        key: ast::PropName::Ident(ast::Ident {
            span: DUMMY_SP,
            sym: "constructor".into(),
            optional: false,
        }),
        is_optional: false,
        params: vec![name_to_swc_param("__")],
        body: Some(ast::BlockStmt {
            span: DUMMY_SP,
            stmts: vec![this_assignment("__")],
        }),
        accessibility: None,
    }
}
