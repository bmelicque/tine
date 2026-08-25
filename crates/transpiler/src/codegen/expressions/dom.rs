use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;

use crate::codegen::{
    expressions::ExpressionResult,
    utils::{create_str, internal_method_call, unsafe_ident_from_str},
    CodeGenerator,
};

impl CodeGenerator<'_, '_> {
    pub fn handle_element_expression(&mut self, node: ir::ElementExpression) -> ExpressionResult {
        let children_results = node
            .children
            .into_iter()
            .map(|c| self.handle_expression(c))
            .collect::<Vec<_>>();
        let (children_prelim, children) = self.extract_necessary(children_results);

        let (attribute_names, attributes_results): (Vec<String>, Vec<ExpressionResult>) = node
            .attributes
            .into_iter()
            .map(|a| (a.name, self.handle_expression(a.value)))
            .unzip();
        let (attributes_prelim, attribute_values) = if children_prelim.is_empty() {
            self.extract_necessary(attributes_results)
        } else {
            self.extract_all(attributes_results)
        };
        let attributes = attribute_values
            .into_iter()
            .zip(attribute_names)
            .map(|(value, name)| {
                swc::PropOrSpread::Prop(Box::new(swc::Prop::KeyValue(swc::KeyValueProp {
                    key: swc::PropName::Ident(unsafe_ident_from_str(&name).into()),
                    value: Box::new(value),
                })))
            })
            .collect();

        let args = vec![
            create_str(&node.tag_name.clone()).into(),
            swc::Expr::Object(swc::ObjectLit {
                span: DUMMY_SP,
                props: attributes,
            })
            .into(),
            swc::Expr::Array(swc::ArrayLit {
                span: DUMMY_SP,
                elems: children.into_iter().map(|c| Some(c.into())).collect(),
            })
            .into(),
        ];
        let expr = internal_method_call("createElement", args);

        ExpressionResult {
            prelim_stmts: vec![attributes_prelim, children_prelim].concat(),
            expr: expr.into(),
        }
    }
}
