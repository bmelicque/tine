use std::collections::HashSet;

use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_symbols::symbols::MethodSymbolId;
use tine_types::types::TypeId;

use crate::codegen::{utils::args_to_string, CodeGenerator};

impl CodeGenerator<'_, '_> {
    pub fn generate_concrete_classes(
        &mut self,
        methods: &[MethodSymbolId],
    ) -> Vec<swc::ClassMember> {
        methods
            .into_iter()
            .map(|m| self.get_method_receiver_args(*m))
            .collect::<HashSet<_>>()
            .into_iter()
            .map(|args| args_to_string(&args))
            .map(|s| child_class_decl(s))
            .collect()
    }

    fn get_method_receiver_args(&self, method: MethodSymbolId) -> Vec<TypeId> {
        let mut entries = self
            .symbols
            .get(method)
            .owner_args
            .iter()
            .collect::<Vec<_>>();
        entries.sort_by(|a, b| a.0.id.cmp(&b.0.id));
        entries.into_iter().map(|(_, t)| *t).collect()
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
