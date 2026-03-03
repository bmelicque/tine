use crate::codegen::{utils::create_ident, CodeGenerator};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::ast;

impl CodeGenerator<'_> {
    pub fn member_expr_to_swc(&mut self, node: &ast::MemberExpression) -> swc::MemberExpr {
        let prop = match node.prop.clone().unwrap() {
            ast::MemberProp::FieldName(i) => {
                swc::MemberProp::Ident(create_ident(i.as_str()).into())
            }
            ast::MemberProp::Index(n) => swc::MemberProp::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(self.expr_to_swc(&n.into())),
            }),
        };

        swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(self.expr_to_swc(node.object.as_ref().unwrap())),
            prop,
        }
    }
}
