use core::panic;

use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::{
    ast::{self, StructLiteralField},
    codegen::{utils::create_str, CodeGenerator},
};

use super::{codegen::TranspilerFlags, utils::create_ident};

impl CodeGenerator {
    pub fn composite_literal_to_swc_expr(&mut self, node: ast::CompositeLiteral) -> swc::Expr {
        match node {
            ast::CompositeLiteral::AnonymousArray(node) => self.anonymous_array_to_swc(node).into(),
            ast::CompositeLiteral::AnonymousStruct(node) => {
                self.anonymous_struct_to_swc(node).into()
            }
            ast::CompositeLiteral::Array(node) => self.array_literal_to_swc_array(node).into(),
            ast::CompositeLiteral::Map(node) => self.map_literal_to_swc_new_map(node).into(),
            ast::CompositeLiteral::Option(node) => {
                self.option_literal_to_swc_new_option(node).into()
            }
            ast::CompositeLiteral::Struct(node) => self.struct_literal_to_swc_new_expr(node).into(),
        }
    }

    pub fn anonymous_array_to_swc(&mut self, node: ast::AnonymousArrayLiteral) -> swc::ArrayLit {
        let elems = node
            .elements
            .into_iter()
            .map(|node| Some(self.expr_or_an_to_swc(node).into()))
            .collect::<Vec<_>>();
        swc::ArrayLit {
            span: DUMMY_SP,
            elems,
        }
    }

    pub fn anonymous_struct_to_swc(&mut self, node: ast::AnonymousStructLiteral) -> swc::NewExpr {
        let swc_args = node
            .fields
            .iter()
            .map(|field| self.expr_to_swc(field.value.clone()).into())
            .collect();
        swc::NewExpr {
            span: DUMMY_SP,
            callee: Box::new("FIXME:".into()),
            args: Some(swc_args),
            type_args: None,
        }
    }

    pub fn array_literal_to_swc_array(&mut self, node: ast::ArrayLiteral) -> swc::ArrayLit {
        let swc_elements = node
            .elements
            .into_iter()
            .map(|node| Some(self.expr_or_an_to_swc(node).into()))
            .collect::<Vec<_>>();
        swc::ArrayLit {
            span: DUMMY_SP,
            elems: swc_elements,
        }
    }

    pub fn map_literal_to_swc_new_map(&mut self, node: ast::MapLiteral) -> swc::NewExpr {
        let swc_args = node
            .entries
            .into_iter()
            .map(|entry| {
                let key = self.expr_to_swc(*entry.key);
                let value = self.expr_or_an_to_swc(*entry.value);
                Some(swc::ExprOrSpread {
                    spread: None,
                    expr: Box::new(swc::Expr::Array(swc::ArrayLit {
                        span: DUMMY_SP,
                        elems: vec![Some(key.into()), Some(value.into())],
                    })),
                })
            })
            .collect::<Vec<_>>();
        swc::NewExpr {
            span: DUMMY_SP,
            callee: Box::new(create_ident("Map").into()),
            args: Some(vec![swc::ExprOrSpread {
                spread: None,
                expr: Box::new(swc::Expr::Array(swc::ArrayLit {
                    span: DUMMY_SP,
                    elems: swc_args,
                })),
            }]),
            type_args: None,
        }
    }

    pub fn option_literal_to_swc_new_option(&mut self, node: ast::OptionLiteral) -> swc::NewExpr {
        self.add_flag(TranspilerFlags::OptionType);

        let exprs = match node.value {
            Some(value) => {
                vec![create_str("Some"), self.expr_or_an_to_swc(*value)]
            }
            None => vec![create_str("None")],
        };
        let args = exprs
            .into_iter()
            .map(|expr| swc::ExprOrSpread {
                spread: None,
                expr: Box::new(expr),
            })
            .collect();

        swc::NewExpr {
            span: DUMMY_SP,
            callee: Box::new(create_ident("__Option").into()),
            args: Some(args),
            type_args: None,
        }
    }

    pub fn struct_literal_to_swc_new_expr(&mut self, node: ast::StructLiteral) -> swc::NewExpr {
        let name = node.ty.name;
        let swc_args = self.get_sorted_args(&name, node.fields);
        swc::NewExpr {
            span: DUMMY_SP,
            callee: Box::new(create_ident(&name).into()),
            args: Some(swc_args),
            type_args: None,
        }
    }

    fn get_sorted_args(
        &mut self,
        name: &str,
        fields: Vec<StructLiteralField>,
    ) -> Vec<swc::ExprOrSpread> {
        let class_def = self.get_class_def(&name).cloned();
        let mut sorted_args = vec![];

        let mut remaining = fields.len();
        let class_def = class_def.unwrap();
        let first = class_def.class.body.iter().next().unwrap();
        let swc::ClassMember::Constructor(constructor) = first else {
            panic!("Expected a constructor in class definition!");
        };

        for param in constructor.params.iter() {
            let swc::ParamOrTsParamProp::Param(param) = param else {
                panic!("Expected a parameter in argument list!");
            };
            let param_name = match param.pat {
                swc::Pat::Ident(ref id) => id.id.sym.to_string(),
                swc::Pat::Assign(ref assign) => assign.left.as_ident().unwrap().id.sym.to_string(),
                _ => panic!("Unexpected pattern type"),
            };

            let field = fields.iter().find(|field| param_name == field.prop);
            let expr = match field {
                Some(field) => {
                    remaining -= 1;
                    self.expr_to_swc(field.value.clone())
                }
                None => swc::Expr::Ident(swc::Ident {
                    span: DUMMY_SP,
                    sym: "undefined".into(),
                    optional: false,
                }),
            };
            sorted_args.push(swc::ExprOrSpread {
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
