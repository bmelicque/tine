use crate::codegen::{utils::create_ident, CodeGenerator};

use mylang_core::ast;

use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use super::{enums::enum_def_to_swc_constructor, literal_alias::literal_alias_to_swc_constructor};

impl CodeGenerator {
    pub fn alias_to_swc(&mut self, node: &ast::TypeAlias) -> Vec<swc::Stmt> {
        let mut super_class = None;
        let body: Vec<swc::ClassMember> = match node.definition.as_ref() {
            ast::TypeDefinition::Enum(def) => {
                for variant in def.variants.iter() {
                    if let ast::VariantDefinition::Struct(ast::StructVariant { def, .. }) = variant
                    {
                        self.register_struct(
                            &format!("{}.{}", node.name, variant.as_name()),
                            &def.fields,
                        );
                    }
                }
                vec![enum_def_to_swc_constructor(def).into()]
            }
            ast::TypeDefinition::Struct(def) => {
                self.register_struct(&node.name, &def.fields);
                vec![self.struct_to_swc_constructor(def).into()]
            }
            ast::TypeDefinition::Type(t) => {
                let ast::Type::Named(named) = t else {
                    panic!("Not implemented yet!")
                };
                if is_literal_type(&named.name) {
                    vec![literal_alias_to_swc_constructor().into()]
                } else {
                    super_class = Some(Box::new(create_ident(&named.name).into()));
                    Vec::new()
                }
            }
        };
        let declaration = swc::ClassDecl {
            declare: false,
            ident: create_ident(&node.name),
            class: Box::new(swc::Class {
                span: DUMMY_SP,
                ctxt: SyntaxContext::empty(),
                body,
                super_class,
                super_type_params: None,
                decorators: vec![],
                type_params: None,
                is_abstract: false,
                implements: vec![],
            }),
        };
        vec![declaration.into()]
    }
}

fn is_literal_type(id: &str) -> bool {
    id == "string" || id == "number" || id == "boolean"
}
