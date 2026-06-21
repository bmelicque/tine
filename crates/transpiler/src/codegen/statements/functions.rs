use std::collections::HashMap;

use crate::codegen::{
    statements::utils::declare_const,
    utils::{generate_constructor_name, ident_from_str, member},
    CodeGenerator,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::{ir, types, SymbolKind, SymbolRef};

impl CodeGenerator<'_> {
    pub fn handle_function_definition(&mut self, node: &ir::FunctionDefinition) -> swc::Stmt {
        let kind = node.name.symbol.borrow().kind.clone();
        match kind {
            SymbolKind::Function { .. } => self.handle_regular_function(node),
            SymbolKind::Method {
                owner, owner_args, ..
            } => self.handle_static_method(node, &owner, &owner_args),
            _ => panic!(),
        }
    }

    fn handle_regular_function(&mut self, node: &ir::FunctionDefinition) -> swc::Stmt {
        swc::Stmt::Decl(swc::Decl::Fn(swc::FnDecl {
            ident: ident_from_str(&node.name.as_name()),
            declare: false,
            function: Box::new(self.handle_function(&node.params, &node.body)),
        }))
    }

    fn handle_static_method(
        &mut self,
        node: &ir::FunctionDefinition,
        ty: &SymbolRef,
        ty_args: &HashMap<types::TypeParam, types::TypeId>,
    ) -> swc::Stmt {
        let constructor = generate_constructor_name(ty, ty_args);

        let name = &node.name.as_name();

        let left =
            swc::AssignTarget::Simple(swc::SimpleAssignTarget::Member(member(constructor, name)));

        let right = Box::new(swc::Expr::Fn(swc::FnExpr {
            ident: None,
            function: Box::new(self.handle_function(&node.params, &node.body)),
        }));

        swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                left,
                right,
                ..Default::default()
            })),
        })
    }

    fn handle_function(&mut self, params: &[ir::Identifier], body: &ir::Block) -> swc::Function {
        let params = params
            .iter()
            .map(|p| swc::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: swc::Pat::Ident(ident_from_str(&p.as_name()).into()),
            })
            .collect();

        swc::Function {
            params,
            body: Some(self.block_to_swc_stmt(body)),
            ..Default::default()
        }
    }

    pub fn handle_method_definition(&mut self, node: &ir::MethodDefinition) -> swc::Stmt {
        let kind = node.name.symbol.borrow().kind.clone();
        let SymbolKind::Method {
            owner, owner_args, ..
        } = kind
        else {
            panic!()
        };
        let constructor = generate_constructor_name(&owner, &owner_args);
        let left = member(
            member(constructor, "prototype").into(),
            &node.name.as_name(),
        )
        .into();

        let mut right = self.with_this(node.receiver.symbol.clone(), |self_| {
            self_.handle_function(&node.params, &node.body)
        });
        right.body.as_mut().unwrap().stmts.insert(
            0,
            swc::Stmt::Decl(declare_const(
                &node.receiver.as_name(),
                swc::ThisExpr { span: DUMMY_SP }.into(),
            )),
        );

        swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                left,
                right: right.into(),
                ..Default::default()
            })),
        })
    }
}
