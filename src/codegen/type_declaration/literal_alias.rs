use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as ast;

use crate::codegen::utils::{create_block_stmt, create_ident};

use super::utils::{name_to_swc_param, this_assignment};

pub fn literal_alias_to_swc_constructor() -> ast::Constructor {
    ast::Constructor {
        span: DUMMY_SP,
        ctxt: SyntaxContext::empty(),
        key: create_ident("constructor").into(),
        is_optional: false,
        params: vec![name_to_swc_param("__")],
        body: Some(create_block_stmt(vec![this_assignment("__")])),
        accessibility: None,
    }
}
