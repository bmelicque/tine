use std::error::Error;

use crate::ast::{Node, Spanned, StructField};
use swc_common::DUMMY_SP;
use swc_ecma_ast as ast;

use super::{expressions::node_to_swc_expr, CodeGenerator};

pub fn type_declaration_to_swc_decl(
    generator: &CodeGenerator,
    node: Node,
) -> Result<Option<ast::ModuleItem>, Box<dyn Error>> {
    let Node::TypeDeclaration {
        name,
        type_params: _,
        def,
    } = node
    else {
        panic!("Expected a type declaration node!");
    };
    let constructor = match def.unwrap().node {
        Node::Struct(fields) => struct_to_swc_constructor(generator, fields),
        Node::SumDef(_) => panic!("Sum types are not supported yet!"),
        Node::TraitDef { .. } => {
            return Ok(None);
        }
        _ => unreachable!("Did not expected this kind of node here!"),
    };
    Ok(Some(ast::ModuleItem::Stmt(
        ast::ClassDecl {
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
        }
        .into(),
    )))
}

pub fn struct_to_swc_constructor(
    generator: &CodeGenerator,
    fields: Vec<Spanned<StructField>>,
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
            stmts: mandatory_fields
                .iter()
                .chain(optional_fields.iter())
                .map(|field| this_assignment(&field.name))
                .collect(),
        }),
        accessibility: None,
    }
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
