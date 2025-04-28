use std::error::Error;

use crate::{
    ast::Node,
    codegen::{expressions::node_to_swc_expr, utils::create_ident, CodeGenerator},
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::{
    literal_alias::literal_alias_to_swc_constructor, struct_type::struct_to_swc_constructor,
    sum_type::sum_def_swc_constructor,
};

pub fn type_declaration_to_swc_decl(
    generator: &mut CodeGenerator,
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
    let mut super_class = None;
    let body: Vec<ast::ClassMember> = match def_node {
        Node::Struct(ref fields) => vec![struct_to_swc_constructor(generator, fields).into()],
        Node::SumDef(variants) => vec![sum_def_swc_constructor(variants).into()],
        Node::TraitDef { .. } => {
            return Ok(None);
        }
        Node::Identifier(id) if is_literal_type(&id) => {
            vec![literal_alias_to_swc_constructor().into()]
        }
        _ => {
            super_class = Some(Box::new(node_to_swc_expr(generator, def_node)));
            Vec::new()
        }
    };
    let declaration = ast::ClassDecl {
        declare: false,
        ident: create_ident(&name),
        class: Box::new(ast::Class {
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
    generator.add_class_def(name, declaration.clone());
    Ok(Some(declaration.into()))
}

fn is_literal_type(id: &str) -> bool {
    id == "string" || id == "number" || id == "boolean"
}
