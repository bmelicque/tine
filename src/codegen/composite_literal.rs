use core::panic;

use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use crate::{
    ast::{AstNode, FieldAssignment, MapEntry, Node, Spanned},
    codegen::{expressions::node_to_swc_expr, CodeGenerator},
};

use super::utils::create_ident;

pub fn array_literal_to_swc_array(
    generator: &CodeGenerator,
    elements: Vec<AstNode>,
) -> ast::ArrayLit {
    let swc_elements = elements
        .into_iter()
        .map(|spanned| Some(node_to_swc_expr(generator, spanned.node).into()))
        .collect::<Vec<_>>();
    ast::ArrayLit {
        span: DUMMY_SP,
        elems: swc_elements,
    }
}

pub fn map_literal_to_swc_new_map(
    generator: &CodeGenerator,
    entries: Vec<Spanned<MapEntry>>,
) -> ast::NewExpr {
    let swc_args = entries
        .into_iter()
        .map(|entry| {
            let key = node_to_swc_expr(generator, entry.node.key.node.clone());
            let value = node_to_swc_expr(generator, entry.node.value.node.clone());
            Some(ast::ExprOrSpread {
                spread: None,
                expr: Box::new(ast::Expr::Array(ast::ArrayLit {
                    span: DUMMY_SP,
                    elems: vec![Some(key.into()), Some(value.into())],
                })),
            })
        })
        .collect::<Vec<_>>();
    ast::NewExpr {
        span: DUMMY_SP,
        callee: Box::new(create_ident("Map").into()),
        args: Some(vec![ast::ExprOrSpread {
            spread: None,
            expr: Box::new(ast::Expr::Array(ast::ArrayLit {
                span: DUMMY_SP,
                elems: swc_args,
            })),
        }]),
        type_args: None,
    }
}

pub fn struct_literal_to_swc_new_expr(
    generator: &CodeGenerator,
    name: Node,
    fields: Vec<Spanned<FieldAssignment>>,
) -> ast::NewExpr {
    let name = get_struct_type_name(&name);
    let swc_args = get_sorted_args(generator, &name, fields);
    ast::NewExpr {
        span: DUMMY_SP,
        callee: Box::new(create_ident(&name).into()),
        args: Some(swc_args),
        type_args: None,
    }
}

fn get_struct_type_name(node: &Node) -> String {
    match node {
        Node::NamedType(id) => id.clone(),
        Node::GenericType { name, .. } => {
            let Node::NamedType(ref id) = name.node else {
                panic!("Expected a named type!");
            };
            id.clone()
        }
        _ => panic!("Expected a type declaration or identifier!"),
    }
}

fn get_sorted_args(
    generator: &CodeGenerator,
    name: &str,
    fields: Vec<Spanned<FieldAssignment>>,
) -> Vec<ast::ExprOrSpread> {
    let class_def = generator.get_class_def(&name);
    let mut sorted_args = vec![];

    let mut remaining = fields.len();
    let first = class_def.unwrap().class.body.iter().next().unwrap();
    let ast::ClassMember::Constructor(constructor) = first else {
        panic!("Expected a constructor in class definition!");
    };

    for param in constructor.params.iter() {
        let ast::ParamOrTsParamProp::Param(param) = param else {
            panic!("Expected a parameter in argument list!");
        };
        let param_name = match param.pat {
            ast::Pat::Ident(ref id) => id.id.sym.to_string(),
            ast::Pat::Assign(ref assign) => assign.left.as_ident().unwrap().id.sym.to_string(),
            _ => panic!("Unexpected pattern type"),
        };

        let field = fields.iter().find(|field| param_name == field.node.name);
        let expr = match field {
            Some(field) => {
                remaining -= 1;
                node_to_swc_expr(generator, field.node.value.node.clone())
            }
            None => ast::Expr::Ident(ast::Ident {
                span: DUMMY_SP,
                sym: "undefined".into(),
                optional: false,
            }),
        };
        sorted_args.push(ast::ExprOrSpread {
            spread: None,
            expr: Box::new(expr),
        });
        if remaining == 0 {
            break;
        }
    }

    sorted_args
}
