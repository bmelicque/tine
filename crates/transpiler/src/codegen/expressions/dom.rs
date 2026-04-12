use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use crate::codegen::{expressions::ExpressionResult, utils::create_ident, CodeGenerator};

use tine_core::ir;

impl CodeGenerator<'_> {
    pub fn handle_element_expression(&mut self, node: &ir::ElementExpression) -> ExpressionResult {
        let children_results = node
            .children
            .iter()
            .map(|c| self.handle_expression(c))
            .collect::<Vec<_>>();
        let (children_prelim, children) = self.extract_necessary(children_results);

        let attributes_results = node
            .attributes
            .iter()
            .map(|a| self.handle_expression(&a.value))
            .collect::<Vec<_>>();
        let (attributes_prelim, attribute_values) = if children_prelim.is_empty() {
            self.extract_necessary(attributes_results)
        } else {
            self.extract_all(attributes_results)
        };
        let attributes = attribute_values
            .into_iter()
            .zip(node.attributes.iter())
            .map(|(value, ir)| {
                swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
                    key: swc::PropName::Ident(create_ident(&ir.name).into()),
                    value: Box::new(value),
                })))
            })
            .collect();

        let expr = swc::CallExpr {
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
                Box::new(swc::Expr::Object(swc::ObjectLit {
                    span: DUMMY_SP,
                    props: attributes,
                }))
                .into(),
                Box::new(swc::Expr::Array(swc::ArrayLit {
                    span: DUMMY_SP,
                    elems: children.into_iter().map(|c| Some(c.into())).collect(),
                }))
                .into(),
            ],
            ..Default::default()
        };

        ExpressionResult {
            prelim_stmts: vec![attributes_prelim, children_prelim].concat(),
            expr: expr.into(),
        }
    }
}
