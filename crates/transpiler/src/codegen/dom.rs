use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::codegen::{utils::create_ident, CodeGenerator};

use tine_core::ast;

impl CodeGenerator<'_> {
    pub fn element_expression_to_swc(&mut self, node: &ast::ElementExpression) -> swc::Expr {
        match node {
            ast::ElementExpression::Element(el) => self.element_to_swc(el),
            ast::ElementExpression::Void(el) => self.void_element_to_swc(el),
        }
    }

    fn element_to_swc(&mut self, node: &ast::Element) -> swc::Expr {
        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(swc::Expr::Ident(create_ident("__"))),
                prop: swc::MemberProp::Ident(create_ident("createElement").into()),
            }))),
            args: vec![
                Box::new(swc::Expr::Lit(swc::Lit::Str(swc::Str {
                    span: DUMMY_SP,
                    value: node.tag_name.clone().into(),
                    raw: None,
                })))
                .into(),
                Box::new(self.attributes_to_swc_object(&node.attributes)).into(),
                Box::new(self.children_to_swc_array(&node.children)).into(),
            ],
            type_args: None,
        })
    }

    fn void_element_to_swc(&mut self, node: &ast::VoidElement) -> swc::Expr {
        swc::Expr::Call(swc::CallExpr {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            callee: swc::Callee::Expr(Box::new(swc::Expr::Ident(create_ident("__createElement")))),
            args: vec![
                Box::new(swc::Expr::Lit(swc::Lit::Str(swc::Str {
                    span: DUMMY_SP,
                    value: node.tag_name.clone().into(),
                    raw: None,
                })))
                .into(),
                Box::new(self.attributes_to_swc_object(&node.attributes)).into(),
            ],
            type_args: None,
        })
    }

    fn attributes_to_swc_object(&mut self, attributes: &Vec<ast::Attribute>) -> swc::Expr {
        let props = attributes
            .into_iter()
            .map(|attr| {
                swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
                    key: swc::PropName::Str(swc::Str {
                        span: DUMMY_SP,
                        value: attr.name.clone().into(),
                        raw: None,
                    }),
                    value: Box::new(self.attribute_value_to_swc(&attr.value)),
                })))
            })
            .collect();
        swc::Expr::Object(swc::ObjectLit {
            span: DUMMY_SP,
            props,
        })
    }

    fn attribute_value_to_swc(&mut self, value: &Option<ast::AttributeValue>) -> swc::Expr {
        match value {
            Some(ast::AttributeValue::String(s)) => swc::Expr::Lit(swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: s.clone()[1..s.len() - 1].into(),
                raw: None,
            })),
            Some(ast::AttributeValue::Expression(e)) => self.expr_to_swc(e),
            None => swc::Expr::Lit(swc::Lit::Bool(swc::Bool {
                span: DUMMY_SP,
                value: true,
            })),
        }
    }

    fn children_to_swc_array(&mut self, children: &Vec<ast::ElementChild>) -> swc::Expr {
        let elems = children
            .into_iter()
            .map(|child| Some(self.child_to_swc(child).into()))
            .collect();
        swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems,
        })
    }

    fn child_to_swc(&mut self, child: &ast::ElementChild) -> swc::Expr {
        match child {
            ast::ElementChild::Element(el) => self.element_to_swc(el),
            ast::ElementChild::VoidElement(el) => self.void_element_to_swc(el),
            ast::ElementChild::Text(t) => swc::Expr::Lit(swc::Lit::Str(swc::Str {
                span: DUMMY_SP,
                value: t.text.clone().into(),
                raw: None,
            })),
            ast::ElementChild::Expression(e) => self.expr_to_swc(e),
        }
    }
}
