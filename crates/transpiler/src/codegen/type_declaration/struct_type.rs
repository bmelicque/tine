use crate::codegen::{
    utils::{create_block_stmt, create_ident},
    CodeGenerator,
};
use mylang_core::ast;
use swc_common::{SyntaxContext, DUMMY_SP};
use swc_ecma_ast as swc;

use super::utils::this_assignment;

impl CodeGenerator {
    pub fn struct_to_swc_constructor(&mut self, node: &ast::StructDefinition) -> swc::Constructor {
        let mandatory_fields: Vec<&ast::StructDefinitionField> = node
            .fields
            .iter()
            .filter(|field| !field.is_optional())
            .collect();
        let optional_fields: Vec<&ast::StructDefinitionField> = node
            .fields
            .iter()
            .filter(|field| field.is_optional())
            .collect();
        let params = mandatory_fields
            .iter()
            .chain(optional_fields.iter())
            .map(self.struct_field_to_swc_param())
            .collect::<Vec<_>>();

        swc::Constructor {
            span: DUMMY_SP,
            ctxt: SyntaxContext::empty(),
            key: create_ident("constructor").into(),
            is_optional: false,
            params,
            body: Some(create_block_stmt(struct_to_swc_constructor_stmts(
                &node.fields,
            ))),
            accessibility: None,
        }
    }

    fn struct_field_to_swc_param<'a>(
        &'a mut self,
    ) -> impl FnMut(&&'a ast::StructDefinitionField) -> swc::ParamOrTsParamProp + 'a {
        move |field| {
            let pattern = match field {
                ast::StructDefinitionField::Mandatory(field) => {
                    swc::Pat::Ident(swc::BindingIdent {
                        id: create_ident(&field.name),
                        type_ann: None,
                    })
                }
                ast::StructDefinitionField::Optional(field) => swc::Pat::Assign(swc::AssignPat {
                    span: DUMMY_SP,
                    left: Box::new(swc::Pat::Ident(swc::BindingIdent {
                        id: create_ident(&field.name),
                        type_ann: None,
                    })),
                    right: Box::new(self.expr_to_swc(&field.default)),
                }),
            };

            swc::ParamOrTsParamProp::Param(swc::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: pattern,
            })
        }
    }
}

fn struct_to_swc_constructor_stmts(fields: &Vec<ast::StructDefinitionField>) -> Vec<swc::Stmt> {
    fields
        .iter()
        .map(|field| this_assignment(&field.as_name()))
        .collect()
}
