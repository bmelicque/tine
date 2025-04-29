use core::panic;

use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use crate::{
    ast::{AstNode, FieldAssignment, MapEntry, Node, Spanned},
    codegen::{utils::create_str, CodeGenerator},
};

use super::{codegen::TranspilerFlags, utils::create_ident};

impl CodeGenerator {
    pub fn array_literal_to_swc_array(&mut self, elements: Vec<AstNode>) -> ast::ArrayLit {
        let swc_elements = elements
            .into_iter()
            .map(|spanned| Some(self.node_to_swc_expr(spanned.node).into()))
            .collect::<Vec<_>>();
        ast::ArrayLit {
            span: DUMMY_SP,
            elems: swc_elements,
        }
    }

    pub fn option_literal_to_swc_new_option(
        &mut self,
        value: Option<Box<AstNode>>,
    ) -> ast::NewExpr {
        self.add_flag(TranspilerFlags::OptionType);

        let exprs = match value {
            Some(value) => {
                vec![create_str("Some"), self.node_to_swc_expr(value.node)]
            }
            None => vec![create_str("None")],
        };

        ast::NewExpr {
            span: DUMMY_SP,
            callee: Box::new(create_ident("__Option").into()),
            args: Some(
                exprs
                    .into_iter()
                    .map(|expr| ast::ExprOrSpread {
                        spread: None,
                        expr: Box::new(expr),
                    })
                    .collect(),
            ),
            type_args: None,
        }
    }

    pub fn map_literal_to_swc_new_map(&mut self, entries: Vec<Spanned<MapEntry>>) -> ast::NewExpr {
        let swc_args = entries
            .into_iter()
            .map(|entry| {
                let key = self.node_to_swc_expr(entry.node.key.node.clone());
                let value = self.node_to_swc_expr(entry.node.value.node.clone());
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
        &mut self,
        name: Node,
        fields: Vec<Spanned<FieldAssignment>>,
    ) -> ast::NewExpr {
        let name = get_struct_type_name(&name);
        let swc_args = self.get_sorted_args(&name, fields);
        ast::NewExpr {
            span: DUMMY_SP,
            callee: Box::new(create_ident(&name).into()),
            args: Some(swc_args),
            type_args: None,
        }
    }

    fn get_sorted_args(
        &mut self,
        name: &str,
        fields: Vec<Spanned<FieldAssignment>>,
    ) -> Vec<ast::ExprOrSpread> {
        let class_def = self.get_class_def(&name).cloned();
        let mut sorted_args = vec![];

        let mut remaining = fields.len();
        let class_def = class_def.unwrap();
        let first = class_def.class.body.iter().next().unwrap();
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
                    self.node_to_swc_expr(field.node.value.node.clone())
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
