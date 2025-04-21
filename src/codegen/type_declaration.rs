use std::error::Error;

use crate::ast::{Node, Spanned, StructField, SumTypeConstructor};
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::{expressions::node_to_swc_expr, CodeGenerator};

pub fn type_declaration_to_swc_decl(
    generator: &CodeGenerator,
    node: Node,
) -> Result<Option<ast::Stmt>, Box<dyn Error>> {
    let Node::TypeDeclaration {
        name,
        type_params: _,
        def,
    } = node
    else {
        panic!("Expected a type declaration node!");
    };
    let def_node = def.unwrap().node;
    let constructor = match def_node {
        Node::Struct(ref fields) => struct_to_swc_constructor(generator, fields),
        Node::SumDef(variants) => sum_def_swc_constructor(generator, variants),
        Node::TraitDef { .. } => {
            return Ok(None);
        }
        _ => unreachable!("Did not expected this kind of node here!"),
    };
    let declaration = ast::ClassDecl {
        declare: false,
        ident: ast::Ident {
            span: DUMMY_SP,
            sym: name.into(),
            optional: false,
        },
        class: Box::new(ast::Class {
            span: DUMMY_SP,
            body: vec![ast::ClassMember::Constructor(constructor)],
            super_class: None,
            super_type_params: None,
            decorators: vec![],
            type_params: None,
            is_abstract: false,
            implements: vec![],
        }),
    };
    Ok(Some(declaration.into()))
}

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
    let mandatory_params = mandatory_fields
        .iter()
        .map(struct_field_to_swc_param(generator))
        .collect::<Vec<_>>();
    let optional_params: Vec<ast::ParamOrTsParamProp> = optional_fields
        .iter()
        .map(struct_field_to_swc_param(generator))
        .collect();

    ast::Constructor {
        span: DUMMY_SP,
        key: ast::PropName::Ident(ast::Ident {
            span: DUMMY_SP,
            sym: "constructor".into(),
            optional: false,
        }),
        is_optional: false,
        params: mandatory_params
            .iter()
            .chain(optional_params.iter())
            .cloned()
            .collect(),
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
        let pattern = match field.optional {
            true => ast::Pat::Assign(ast::AssignPat {
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
            }),
            false => ast::Pat::Ident(ast::BindingIdent {
                id: ast::Ident {
                    span: DUMMY_SP,
                    sym: field.name.clone().into(),
                    optional: false,
                },
                type_ann: None,
            }),
        };

        ast::ParamOrTsParamProp::Param(ast::Param {
            span: DUMMY_SP,
            decorators: vec![],
            pat: pattern,
        })
    }
}

fn sum_def_swc_constructor(
    generator: &CodeGenerator,
    variants: Vec<SumTypeConstructor>,
) -> ast::Constructor {
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
        params: vec![get_sum_tag_param(), get_sum_values_param()],
        body: Some(ast::BlockStmt {
            span: DUMMY_SP,
            stmts,
        }),
        accessibility: None,
    }
}

fn get_sum_tag_param() -> ast::ParamOrTsParamProp {
    ast::ParamOrTsParamProp::Param(ast::Param {
        span: DUMMY_SP,
        decorators: vec![],
        pat: ast::Pat::Ident(ast::BindingIdent {
            id: ast::Ident {
                span: DUMMY_SP,
                sym: "__".into(),
                optional: false,
            },
            type_ann: None,
        }),
    })
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

// Specifically, create an assignemnent like `this.field_name = field_name;`
fn this_assignment(field_name: &str) -> ast::Stmt {
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
                    sym: field_name.into(),
                    optional: false,
                }),
            }))),
            right: Box::new(ast::Expr::Ident(ast::Ident {
                span: DUMMY_SP,
                sym: field_name.into(),
                optional: false,
            })),
        })),
    })
}
