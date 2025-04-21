use crate::ast::{Node, SumTypeConstructor};
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::{
    struct_type::struct_to_swc_constructor_stmts,
    utils::{name_to_swc_param, this_assignment},
};

pub fn sum_def_swc_constructor(variants: Vec<SumTypeConstructor>) -> ast::Constructor {
    let stmts = match variants_to_swc_switch(variants) {
        Some(swc_switch) => vec![this_assignment("__"), swc_switch.into()],
        None => vec![this_assignment("__")],
    };
    ast::Constructor {
        span: DUMMY_SP,
        key: ast::PropName::Ident(ast::Ident {
            span: DUMMY_SP,
            sym: "constructor".into(),
            optional: false,
        }),
        is_optional: false,
        params: vec![name_to_swc_param("__"), get_sum_values_param()],
        body: Some(ast::BlockStmt {
            span: DUMMY_SP,
            stmts,
        }),
        accessibility: None,
    }
}

fn get_sum_values_param() -> ast::ParamOrTsParamProp {
    ast::ParamOrTsParamProp::Param(ast::Param {
        span: DUMMY_SP,
        decorators: vec![],
        pat: ast::Pat::Rest(ast::RestPat {
            span: DUMMY_SP,
            arg: Box::new(ast::Pat::Ident(ast::BindingIdent {
                id: ast::Ident {
                    span: DUMMY_SP,
                    sym: "values".into(),
                    optional: false,
                },
                type_ann: None,
            })),
            dot3_token: DUMMY_SP,
            type_ann: None,
        }),
    })
}

fn variants_to_swc_switch(variants: Vec<SumTypeConstructor>) -> Option<ast::SwitchStmt> {
    let cases: Vec<ast::SwitchCase> = variants
        .iter()
        .filter(|variant| variant.param.is_some())
        .map(variant_to_swc_switch_case)
        .collect();
    if cases.is_empty() {
        return None;
    }
    Some(ast::SwitchStmt {
        span: DUMMY_SP,
        discriminant: Box::new(ast::Expr::Ident(ast::Ident {
            span: DUMMY_SP,
            sym: "__".into(),
            optional: false,
        })),
        cases,
    })
}

fn variant_to_swc_switch_case(variant: &SumTypeConstructor) -> ast::SwitchCase {
    ast::SwitchCase {
        span: DUMMY_SP,
        test: Some(Box::new(ast::Expr::Lit(ast::Lit::Str(ast::Str {
            span: DUMMY_SP,
            value: variant.name.clone().into(),
            raw: None,
        })))),
        cons: match &variant.param.as_ref().unwrap().node {
            Node::Struct(ref fields) => {
                let mut stmts = struct_to_swc_constructor_stmts(fields);
                stmts.push(ast::Stmt::Break(ast::BreakStmt {
                    span: DUMMY_SP,
                    label: None,
                }));
                stmts
            }
            _ => {
                vec![
                    this_sum_default_assignement(),
                    ast::Stmt::Break(ast::BreakStmt {
                        span: DUMMY_SP,
                        label: None,
                    }),
                ]
            }
        },
    }
}

fn this_sum_default_assignement() -> ast::Stmt {
    ast::Stmt::Expr(ast::ExprStmt {
        span: DUMMY_SP,
        expr: Box::new(ast::Expr::Assign(ast::AssignExpr {
            span: DUMMY_SP,
            op: ast::AssignOp::Assign,
            left: ast::PatOrExpr::Expr(Box::new(ast::Expr::Member(ast::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(ast::Expr::This(ast::ThisExpr { span: DUMMY_SP })),
                prop: ast::MemberProp::Ident(ast::Ident {
                    span: DUMMY_SP,
                    sym: "_0".into(),
                    optional: false,
                }),
            }))),
            right: Box::new(ast::Expr::Member(ast::MemberExpr {
                span: DUMMY_SP,
                obj: Box::new(ast::Expr::Ident(ast::Ident {
                    span: DUMMY_SP,
                    sym: "values".into(),
                    optional: false,
                })),
                prop: ast::MemberProp::Computed(ast::ComputedPropName {
                    span: DUMMY_SP,
                    expr: Box::new(ast::Expr::Lit(ast::Lit::Num(ast::Number {
                        span: DUMMY_SP,
                        value: 0.0,
                        raw: None,
                    }))),
                }),
            })),
        })),
    })
}
