use crate::{
    ast::{Spanned, StructField},
    codegen::{utils::create_ident, CodeGenerator},
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::utils::this_assignment;

impl CodeGenerator {
    pub fn struct_to_swc_constructor(
        &mut self,
        fields: &Vec<Spanned<StructField>>,
    ) -> ast::Constructor {
        let mandatory_fields: Vec<&StructField> = fields
            .iter()
            .map(|spanned| &spanned.node)
            .filter(|f| !f.optional)
            .collect();
        let optional_fields: Vec<&StructField> = fields
            .iter()
            .map(|spanned| &spanned.node)
            .filter(|f| f.optional)
            .collect();
        let params = mandatory_fields
            .iter()
            .chain(optional_fields.iter())
            .map(self.struct_field_to_swc_param())
            .collect::<Vec<_>>();

        ast::Constructor {
            span: DUMMY_SP,
            key: create_ident("constructor").into(),
            is_optional: false,
            params,
            body: Some(ast::BlockStmt {
                span: DUMMY_SP,
                stmts: struct_to_swc_constructor_stmts(fields),
            }),
            accessibility: None,
        }
    }

    fn struct_field_to_swc_param<'a>(
        &'a mut self,
    ) -> impl FnMut(&&'a StructField) -> ast::ParamOrTsParamProp + 'a {
        move |field| {
            let pattern = if field.optional {
                ast::Pat::Assign(ast::AssignPat {
                    span: DUMMY_SP,
                    left: Box::new(ast::Pat::Ident(ast::BindingIdent {
                        id: create_ident(&field.name),
                        type_ann: None,
                    })),
                    right: Box::new(
                        self.node_to_swc_expr(field.def.as_ref().unwrap().node.clone()),
                    ),
                })
            } else {
                ast::Pat::Ident(ast::BindingIdent {
                    id: create_ident(&field.name),
                    type_ann: None,
                })
            };

            ast::ParamOrTsParamProp::Param(ast::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: pattern,
            })
        }
    }
}

fn struct_to_swc_constructor_stmts(fields: &Vec<Spanned<StructField>>) -> Vec<ast::Stmt> {
    fields
        .iter()
        .map(|spanned| &spanned.node)
        .map(|field| this_assignment(&field.name))
        .collect()
}
