use crate::{
    ast::{self, StructDefinitionField, VariantDefinition},
    codegen::utils::create_ident,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;

use super::utils::{name_to_swc_param, this_assignment};

pub fn enum_def_to_swc_constructor(node: ast::EnumDefinition) -> swc::Constructor {
    let stmts = match variants_to_swc_switch(node.variants) {
        Some(swc_switch) => vec![this_assignment("__"), swc_switch.into()],
        None => vec![this_assignment("__")],
    };
    swc::Constructor {
        span: DUMMY_SP,
        key: create_ident("constructor").into(),
        is_optional: false,
        params: vec![name_to_swc_param("__"), get_sum_values_param()],
        body: Some(swc::BlockStmt {
            span: DUMMY_SP,
            stmts,
        }),
        accessibility: None,
    }
}

fn get_sum_values_param() -> swc::ParamOrTsParamProp {
    swc::ParamOrTsParamProp::Param(swc::Param {
        span: DUMMY_SP,
        decorators: vec![],
        pat: swc::Pat::Rest(swc::RestPat {
            span: DUMMY_SP,
            arg: Box::new(swc::Pat::Ident(swc::BindingIdent {
                id: create_ident("values"),
                type_ann: None,
            })),
            dot3_token: DUMMY_SP,
            type_ann: None,
        }),
    })
}

fn variants_to_swc_switch(variants: Vec<VariantDefinition>) -> Option<swc::SwitchStmt> {
    let cases: Vec<swc::SwitchCase> = variants
        .iter()
        .filter(|variant| !variant.is_unit())
        .map(variant_to_swc_switch_case)
        .collect();
    if cases.is_empty() {
        return None;
    }
    Some(swc::SwitchStmt {
        span: DUMMY_SP,
        discriminant: Box::new(create_ident("__").into()),
        cases,
    })
}

fn variant_to_swc_switch_case(variant: &VariantDefinition) -> swc::SwitchCase {
    swc::SwitchCase {
        span: DUMMY_SP,
        test: Some(Box::new(swc::Expr::Lit(swc::Lit::Str(swc::Str {
            span: DUMMY_SP,
            value: variant.as_name().into(),
            raw: None,
        })))),
        cons: match variant {
            ast::VariantDefinition::Struct(variant) => {
                let mut stmts = variant_to_swc_constructor_stmts(&variant.def.fields);
                stmts.push(swc::Stmt::Break(swc::BreakStmt {
                    span: DUMMY_SP,
                    label: None,
                }));
                stmts
            }
            ast::VariantDefinition::Tuple(variant) => {
                let mut stmts: Vec<swc::Stmt> = variant
                    .elements
                    .iter()
                    .enumerate()
                    .map(|(i, _)| this_assignement_from_values(&format!("_{}", i), i as f64))
                    .collect();
                stmts.push(swc::Stmt::Break(swc::BreakStmt {
                    span: DUMMY_SP,
                    label: None,
                }));
                stmts
            }
            ast::VariantDefinition::Unit(_) => {
                vec![swc::Stmt::Break(swc::BreakStmt {
                    span: DUMMY_SP,
                    label: None,
                })]
            }
        },
    }
}

fn variant_to_swc_constructor_stmts(fields: &Vec<StructDefinitionField>) -> Vec<swc::Stmt> {
    fields
        .iter()
        .enumerate()
        .map(|(index, field)| this_assignement_from_values(&field.as_name(), index as f64))
        .collect()
}

fn this_assignement_from_values(name: &str, index: f64) -> swc::Stmt {
    let span = DUMMY_SP;
    let expr = Box::new(swc::Expr::Assign(swc::AssignExpr {
        span: DUMMY_SP,
        op: swc::AssignOp::Assign,
        left: swc::PatOrExpr::Expr(Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(swc::ThisExpr { span: DUMMY_SP }.into()),
            prop: create_ident(name).into(),
        }))),
        right: Box::new(swc::Expr::Member(swc::MemberExpr {
            span: DUMMY_SP,
            obj: Box::new(create_ident("values").into()),
            prop: swc::MemberProp::Computed(swc::ComputedPropName {
                span: DUMMY_SP,
                expr: Box::new(swc::Expr::Lit(swc::Lit::Num(swc::Number {
                    span: DUMMY_SP,
                    value: index,
                    raw: None,
                }))),
            }),
        })),
    }));
    swc::ExprStmt { span, expr }.into()
}
