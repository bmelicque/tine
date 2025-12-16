use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::codegen::{utils::create_str, CodeGenerator};

use mylang_core::ast;

use super::utils::create_ident;

impl CodeGenerator<'_> {
    pub fn composite_literal_to_swc_expr(&mut self, node: &ast::CompositeLiteral) -> swc::Expr {
        match node {
            ast::CompositeLiteral::AnonymousStruct(node) => {
                self.anonymous_struct_to_swc(node).into()
            }
            ast::CompositeLiteral::Array(node) => self.array_literal_to_swc_array(node).into(),
            ast::CompositeLiteral::Map(node) => self.map_literal_to_swc_new_map(node).into(),
            ast::CompositeLiteral::Option(node) => {
                self.option_literal_to_swc_new_option(node).into()
            }
            ast::CompositeLiteral::Struct(node) => self.struct_literal_to_swc_new_expr(node).into(),
            ast::CompositeLiteral::Variant(node) => self.variant_literal_to_swc(node).into(),
        }
    }

    pub fn anonymous_struct_to_swc(&mut self, node: &ast::AnonymousStructLiteral) -> swc::NewExpr {
        let swc_args = node
            .fields
            .iter()
            .map(|field| self.expr_to_swc(&field.value).into())
            .collect();
        swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new("FIXME:".into()),
            args: Some(swc_args),
            type_args: None,
        }
    }

    pub fn array_literal_to_swc_array(&mut self, node: &ast::ArrayLiteral) -> swc::ArrayLit {
        let swc_elements = node
            .elements
            .iter()
            .map(|node| Some(self.expr_or_an_to_swc(&node).into()))
            .collect::<Vec<_>>();
        swc::ArrayLit {
            span: DUMMY_SP,
            elems: swc_elements,
        }
    }

    pub fn map_literal_to_swc_new_map(&mut self, node: &ast::MapLiteral) -> swc::NewExpr {
        let swc_args = node
            .entries
            .iter()
            .map(|entry| {
                let key = self.expr_to_swc(&entry.key);
                let value = self.expr_or_an_to_swc(&entry.value);
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
            ctxt: SyntaxContext::empty(),
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

    pub fn option_literal_to_swc_new_option(&mut self, node: &ast::OptionLiteral) -> swc::NewExpr {
        let exprs = match &node.value {
            Some(value) => {
                vec![create_str("Some"), self.expr_or_an_to_swc(value)]
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
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Ident(create_ident("__"))),
                prop: swc::MemberProp::Ident(create_ident("Option").into()),
            })),
            args: Some(args),
            type_args: None,
        }
    }

    pub fn struct_literal_to_swc_new_expr(&mut self, node: &ast::StructLiteral) -> swc::NewExpr {
        let name = &node.ty.name;
        let swc_args = self.get_sorted_args(name, &node.fields);
        swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(create_ident(&name).into()),
            args: Some(swc_args),
            type_args: None,
        }
    }

    fn get_sorted_args(
        &mut self,
        name: &str,
        fields: &Vec<ast::StructLiteralField>,
    ) -> Vec<swc::ExprOrSpread> {
        let mut remaining = fields.len();
        let mut sorted_args = vec![];
        let params = self.find(&name.to_string()).unwrap().clone();
        for param in params {
            let field = fields.iter().find(|field| *param == field.prop);
            let expr = match field {
                Some(field) => {
                    remaining -= 1;
                    self.expr_to_swc(&field.value)
                }
                None => swc::Expr::Ident(create_ident("undefined")),
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

    fn variant_literal_to_swc(&mut self, node: &ast::VariantLiteral) -> swc::NewExpr {
        let name = &node.ty.name;

        let mut args = Vec::<swc::ExprOrSpread>::new();
        args.push(
            swc::Expr::Lit(swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: node.name.clone().into(),
                raw: None,
            }))
            .into(),
        );
        match &node.body {
            Some(ast::VariantLiteralBody::Struct(body)) => {
                let name = format!("{}.{}", name, node.name.clone());
                args.extend(self.get_sorted_args(&name, body));
            }
            Some(ast::VariantLiteralBody::Tuple(body)) => {
                for arg in body {
                    args.push(self.expr_or_an_to_swc(arg).into());
                }
            }
            None => {}
        }

        swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(create_ident(&name).into()),
            args: Some(args),
            type_args: None,
        }
    }

    pub fn some(&mut self, expr: swc::Expr) -> swc::NewExpr {
        let exprs = vec![create_str("Some"), expr];
        let args = exprs
            .into_iter()
            .map(|expr| swc::ExprOrSpread {
                spread: None,
                expr: Box::new(expr),
            })
            .collect();

        swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Ident(create_ident("__"))),
                prop: swc::MemberProp::Ident(create_ident("Option").into()),
            })),
            args: Some(args),
            type_args: None,
        }
    }

    pub fn none(&mut self) -> swc::NewExpr {
        let args = vec![swc::ExprOrSpread {
            spread: None,
            expr: Box::new(create_str("None")),
        }];

        swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Ident(create_ident("__"))),
                prop: swc::MemberProp::Ident(create_ident("Option").into()),
            })),
            args: Some(args),
            type_args: None,
        }
    }
}
