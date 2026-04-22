use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::codegen::utils::ident_from_str;

pub fn declare_const(name: &str, value: swc::Expr) -> swc::Decl {
    swc::Decl::Var(Box::new(swc::VarDecl {
        kind: swc::VarDeclKind::Const,
        decls: vec![swc::VarDeclarator {
            span: DUMMY_SP,
            name: swc::Pat::Ident(ident_from_str(name).into()),
            init: Some(Box::new(value)),
            definite: false,
        }],
        ..Default::default()
    }))
}
