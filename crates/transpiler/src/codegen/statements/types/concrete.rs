use std::collections::HashSet;

use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::{types::TypeId, SymbolKind, SymbolRef};

use crate::codegen::{statements::utils::args_to_string, CodeGenerator};

impl CodeGenerator<'_> {
    pub fn generate_concrete_classes(&mut self, methods: &[SymbolRef]) -> Vec<swc::ClassMember> {
        methods
            .iter()
            .filter_map(|m| get_method_receiver_args(m))
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|args| args_to_string(&args))
            .map(|s| child_class_decl(s))
            .collect()
    }
}

fn get_method_receiver_args(method: &SymbolRef) -> Option<Vec<TypeId>> {
    match &method.borrow().kind {
        SymbolKind::Method { owner_args, .. } => Some(owner_args.clone()),
        _ => None,
    }
}

// `static ID = class extends this {}`
fn child_class_decl(id: String) -> swc::ClassMember {
    let class = swc::ClassExpr {
        ident: None,
        class: Box::new(swc::Class {
            super_class: Some(Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP }))),
            ..Default::default()
        }),
    };

    swc::ClassMember::ClassProp(swc::ClassProp {
        key: swc::PropName::Ident(swc::IdentName::new(id.into(), DUMMY_SP)),
        value: Some(class.into()),
        is_static: true,
        ..Default::default()
    })
}
