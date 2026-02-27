use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::codegen::{utils::create_str, CodeGenerator};

use tine_core::{ast, types};

use super::utils::create_ident;

impl CodeGenerator<'_> {
    pub fn constructor_literal_to_swc_expr(&mut self, node: &ast::ConstructorLiteral) -> swc::Expr {
        if let types::Type::Map(_) = self.get_type_at(node.loc).unwrap() {
            return self.map_literal_to_swc_new_map(node).into();
        }

        match &node.constructor {
            ast::Constructor::Variant(variant) => self.variant_literal_to_swc(node, variant).into(),
            ast::Constructor::Named(name) => self.struct_literal_to_swc_new_expr(node, name).into(),
            _ => panic!(),
        }
    }

    pub fn map_literal_to_swc_new_map(&mut self, node: &ast::ConstructorLiteral) -> swc::NewExpr {
        let Some(ast::ConstructorBody::Struct(body)) = &node.body else {
            panic!("This should not be allowed after checking")
        };

        let swc_args = body
            .fields
            .iter()
            .map(|f| {
                let key = match f.key.as_ref().unwrap() {
                    ast::ConstructorKey::MapKey(expr) => self.expr_to_swc(expr),
                    _ => panic!("Should've been catch during checking"),
                };
                let value = self.expr_to_swc(f.value.as_ref().unwrap());
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

    pub fn struct_literal_to_swc_new_expr(
        &mut self,
        node: &ast::ConstructorLiteral,
        name: &ast::NamedType,
    ) -> swc::NewExpr {
        let name = &name.name;
        let swc_args = self.constructor_body_to_swc(name, node.body.as_ref().unwrap());
        swc::NewExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: Box::new(create_ident(&name).into()),
            args: Some(swc_args),
            type_args: None,
        }
    }

    fn constructor_body_to_swc(
        &mut self,
        name: &str,
        body: &ast::ConstructorBody,
    ) -> Vec<swc::ExprOrSpread> {
        match body {
            ast::ConstructorBody::Struct(st) => self.get_sorted_args(name, &st.fields),
            ast::ConstructorBody::Tuple(t) => t
                .elements
                .iter()
                .map(|e| swc::ExprOrSpread {
                    spread: None,
                    expr: Box::new(self.expr_to_swc(e)),
                })
                .collect(),
        }
    }

    fn get_sorted_args(
        &mut self,
        name: &str,
        fields: &Vec<ast::ConstructorField>,
    ) -> Vec<swc::ExprOrSpread> {
        let mut remaining = fields.len();
        let mut sorted_args = vec![];
        let params = self.find(&name.to_string()).unwrap().clone();
        for param in params {
            let field = fields.iter().find(|field| {
                let key = match &field.key {
                    Some(ast::ConstructorKey::Name(id)) => id.as_str(),
                    _ => panic!(),
                };
                param == key
            });
            let expr = match field {
                Some(field) => {
                    remaining -= 1;
                    self.expr_to_swc(field.value.as_ref().unwrap())
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

    fn variant_literal_to_swc(
        &mut self,
        node: &ast::ConstructorLiteral,
        variant: &ast::VariantConstructor,
    ) -> swc::NewExpr {
        let name = &variant.enum_name.name;

        let mut args = Vec::<swc::ExprOrSpread>::new();
        args.push(
            swc::Expr::Lit(swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: name.to_owned().into(),
                raw: None,
            }))
            .into(),
        );
        let swc_name = format!(
            "{}.{}",
            name,
            variant.variant_name.as_ref().unwrap().as_str()
        );
        if let Some(body) = &node.body {
            let swc_args = self.constructor_body_to_swc(&swc_name, body);
            args.extend(swc_args);
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
