use crate::{
    ast,
    codegen::{utils::create_ident, CodeGenerator},
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use super::{enums::enum_def_to_swc_constructor, literal_alias::literal_alias_to_swc_constructor};

impl CodeGenerator {
    pub fn alias_to_swc(&mut self, node: ast::TypeAlias) -> Option<swc::Stmt> {
        let mut super_class = None;
        let body: Vec<swc::ClassMember> = match *node.definition {
            ast::TypeDefinition::Enum(node) => vec![enum_def_to_swc_constructor(node).into()],
            ast::TypeDefinition::Struct(node) => {
                vec![self.struct_to_swc_constructor(node).into()]
            }
            ast::TypeDefinition::Trait(_) => return None,
            ast::TypeDefinition::Type(t) => {
                let ast::Type::Named(named) = t else {
                    panic!("Not implemented yet!")
                };
                if is_literal_type(&named.name) {
                    vec![literal_alias_to_swc_constructor().into()]
                } else {
                    super_class = Some(Box::new(
                        swc::Ident {
                            span: DUMMY_SP,
                            sym: named.name.into(),
                            optional: false,
                        }
                        .into(),
                    ));
                    Vec::new()
                }
            }
        };
        let declaration = swc::ClassDecl {
            declare: false,
            ident: create_ident(&node.name),
            class: Box::new(swc::Class {
                span: DUMMY_SP,
                body,
                super_class,
                super_type_params: None,
                decorators: vec![],
                type_params: None,
                is_abstract: false,
                implements: vec![],
            }),
        };
        self.add_class_def(node.name, declaration.clone());
        Some(declaration.into())
    }
}

fn is_literal_type(id: &str) -> bool {
    id == "string" || id == "number" || id == "boolean"
}
