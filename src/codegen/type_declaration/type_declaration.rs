use std::error::Error;

use crate::{ast::Node, codegen::CodeGenerator};
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::{struct_type::struct_to_swc_constructor, sum_type::sum_def_swc_constructor};

pub fn type_declaration_to_swc_decl(
    generator: &CodeGenerator,
    node: Node,
) -> Result<Option<ast::Stmt>, Box<dyn Error>> {
    let Node::TypeDeclaration {
        name,
        type_params: _,
        def,
    } = node
    else {
        panic!("Expected a type declaration node!");
    };
    let def_node = def.unwrap().node;
    let constructor = match def_node {
        Node::Struct(ref fields) => struct_to_swc_constructor(generator, fields),
        Node::SumDef(variants) => sum_def_swc_constructor(variants),
        Node::TraitDef { .. } => {
            return Ok(None);
        }
        _ => unreachable!("Did not expected this kind of node here!"),
    };
    let declaration = ast::ClassDecl {
        declare: false,
        ident: ast::Ident {
            span: DUMMY_SP,
            sym: name.into(),
            optional: false,
        },
        class: Box::new(ast::Class {
            span: DUMMY_SP,
            body: vec![ast::ClassMember::Constructor(constructor)],
            super_class: None,
            super_type_params: None,
            decorators: vec![],
            type_params: None,
            is_abstract: false,
            implements: vec![],
        }),
    };
    Ok(Some(declaration.into()))
}
