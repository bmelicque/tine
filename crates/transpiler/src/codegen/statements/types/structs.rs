use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;
use tine_symbols::symbols::*;

use crate::codegen::{
    statements::types::utils::this_assignment, utils::ident_from_str, CodeGenerator,
};

impl CodeGenerator<'_, '_> {
    pub(crate) fn struct_def_to_swc(&mut self, node: ir::StructDefinition) -> swc::ClassDecl {
        let members = &self.symbols.get(node.symbol).members;

        let constructor = self.struct_fields_to_swc_constructor(members);
        let get = self.make_struct_getter(members);
        let set = self.make_setter(members);
        let mut body = vec![constructor.into(), get.into(), set.into()];

        let methods = &self.symbols.get(node.symbol).methods;
        let child_classes = self.generate_concrete_classes(methods);
        body.extend(child_classes);

        let class = swc::Class {
            span: DUMMY_SP,
            body,
            ..Default::default()
        };

        let name = &self.symbols.get(node.symbol).name;
        swc::ClassDecl {
            ident: ident_from_str(name),
            declare: false,
            class: Box::new(class),
        }
    }

    fn struct_fields_to_swc_constructor(&mut self, body: &[MemberSymbolId]) -> swc::Constructor {
        let params = body
            .iter()
            .map(|symbol| {
                let name = &self.symbols.get(*symbol).name;
                swc::ParamOrTsParamProp::Param(swc::Param {
                    span: DUMMY_SP,
                    decorators: vec![],
                    pat: swc::Pat::Ident(ident_from_str(name).into()),
                })
            })
            .collect();

        let body = swc::BlockStmt {
            stmts: body
                .into_iter()
                .map(|symbol| {
                    let name = &self.symbols.get(*symbol).name;
                    this_assignment(ident_from_str(name), ident_from_str(name).into())
                })
                .collect(),
            ..Default::default()
        };

        swc::Constructor {
            key: swc::PropName::Ident(ident_from_str("constructor").into()),
            params,
            body: Some(body),
            ..Default::default()
        }
    }

    fn make_struct_getter(&mut self, fields: &[MemberSymbolId]) -> swc::ClassMethod {
        let args = fields.iter().map(|f| self.get_field(*f).into()).collect();

        let stmt = swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::New(swc::NewExpr {
                callee: Box::new(swc::Expr::This(swc::ThisExpr { span: DUMMY_SP })),
                args: Some(args),
                ..Default::default()
            })),
        });

        swc::ClassMethod {
            key: swc::PropName::Ident(ident_from_str("$get").into()),
            function: Box::new(swc::Function {
                body: Some(swc::BlockStmt {
                    stmts: vec![stmt],
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    fn make_setter(&mut self, fields: &[MemberSymbolId]) -> swc::ClassMethod {
        let stmts = self.set_fields(fields);

        swc::ClassMethod {
            key: swc::PropName::Ident(ident_from_str("$set").into()),
            function: Box::new(swc::Function {
                body: Some(swc::BlockStmt {
                    stmts,
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}
