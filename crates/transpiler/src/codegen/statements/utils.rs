use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::types::TypeId;

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

/// Convert a `Vec<TypeId>` into a unique `String` that will not collide with
/// user-defined names
pub fn args_to_string(args: &[TypeId]) -> String {
    let str = args
        .into_iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join("_");
    format!("${}", str)
}

pub fn member(object: swc::Expr, prop: &str) -> swc::MemberExpr {
    swc::MemberExpr {
        span: DUMMY_SP,
        obj: Box::new(object),
        prop: swc::MemberProp::Ident(ident_from_str(prop).into()),
    }
}
