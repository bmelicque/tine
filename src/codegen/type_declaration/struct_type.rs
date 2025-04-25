use crate::{
    ast::{Spanned, StructField},
    codegen::{expressions::node_to_swc_expr, CodeGenerator},
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::utils::this_assignment;

pub fn struct_to_swc_constructor(
    generator: &CodeGenerator,
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
        .map(struct_field_to_swc_param(generator))
        .collect::<Vec<_>>();

    ast::Constructor {
        span: DUMMY_SP,
        key: ast::PropName::Ident(ast::Ident {
            span: DUMMY_SP,
            sym: "constructor".into(),
            optional: false,
        }),
        is_optional: false,
        params,
        body: Some(ast::BlockStmt {
            span: DUMMY_SP,
            stmts: struct_to_swc_constructor_stmts(fields),
        }),
        accessibility: None,
    }
}

fn struct_to_swc_constructor_stmts(fields: &Vec<Spanned<StructField>>) -> Vec<ast::Stmt> {
    fields
        .iter()
        .map(|spanned| &spanned.node)
        .map(|field| this_assignment(&field.name))
        .collect()
}

fn struct_field_to_swc_param<'a>(
    generator: &'a CodeGenerator,
) -> impl Fn(&&'a StructField) -> ast::ParamOrTsParamProp + 'a {
    move |field| {
        let pattern = if field.optional {
            ast::Pat::Assign(ast::AssignPat {
                span: DUMMY_SP,
                left: Box::new(ast::Pat::Ident(ast::BindingIdent {
                    id: ast::Ident {
                        span: DUMMY_SP,
                        sym: field.name.clone().into(),
                        optional: false,
                    },
                    type_ann: None,
                })),
                right: Box::new(
                    node_to_swc_expr(generator, field.def.as_ref().unwrap().node.clone()).unwrap(),
                ),
            })
        } else {
            ast::Pat::Ident(ast::BindingIdent {
                id: ast::Ident {
                    span: DUMMY_SP,
                    sym: field.name.clone().into(),
                    optional: false,
                },
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
