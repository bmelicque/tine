use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use crate::codegen::{utils::create_ident, CodeGenerator};

use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn element_expression_to_swc(&mut self, node: &ir::ElementExpression) -> swc::Expr {
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

    fn attributes_to_swc_object(&mut self, attributes: &Vec<ir::Attribute>) -> swc::Expr {
        let props = attributes
            .into_iter()
            .map(|attr| {
                swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
                    key: swc::PropName::Str(swc::Str {
                        span: DUMMY_SP,
                        value: attr.name.clone().into(),
                        raw: None,
                    }),
                    value: Box::new(self.expr_to_swc(&attr.value)),
                })))
            })
            .collect();
        swc::Expr::Object(swc::ObjectLit {
            span: DUMMY_SP,
            props,
        })
    }

    fn children_to_swc_array(&mut self, children: &Vec<ir::Expression>) -> swc::Expr {
        let elems = children
            .into_iter()
            .map(|child| Some(self.expr_to_swc(child).into()))
            .collect();
        swc::Expr::Array(swc::ArrayLit {
            span: DUMMY_SP,
            elems,
        })
    }
}
