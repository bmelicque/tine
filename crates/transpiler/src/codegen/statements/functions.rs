use crate::codegen::{
    statements::utils::{args_to_string, member},
    utils::ident_from_str,
    CodeGenerator,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_core::{ir, types::TypeId, SymbolKind, SymbolRef};

impl CodeGenerator<'_> {
    pub fn handle_function_definition(&mut self, node: &ir::FunctionDefinition) -> swc::Stmt {
        let kind = node.name.symbol.borrow().kind.clone();
        match kind {
            SymbolKind::Function { .. } => self.handle_regular_function(node),
            SymbolKind::Method {
                owner,
                owner_args,
                has_receiver,
                ..
            } => self.handle_method_definition(node, &owner, &owner_args, !has_receiver),
            _ => panic!(),
        }
    }

    fn handle_regular_function(&mut self, node: &ir::FunctionDefinition) -> swc::Stmt {
        swc::Stmt::Decl(swc::Decl::Fn(swc::FnDecl {
            ident: ident_from_str(&node.name.as_name()),
            declare: false,
            function: Box::new(self.handle_function(node)),
        }))
    }

    fn handle_method_definition(
        &mut self,
        node: &ir::FunctionDefinition,
        ty: &SymbolRef,
        ty_args: &[TypeId],
        is_static: bool,
    ) -> swc::Stmt {
        let ty = ident_from_str(&ty.as_name());
        let constructor: swc::Expr = match ty_args.len() {
            0 => ty.into(),
            _ => member(ty.into(), &args_to_string(ty_args)).into(),
        };

        let name = &node.name.as_name();

        let left = match is_static {
            true => member(constructor, name),
            false => member(member(constructor, "prototype").into(), name),
        };
        let left = swc::AssignTarget::Simple(swc::SimpleAssignTarget::Member(left));

        let right = Box::new(swc::Expr::Fn(swc::FnExpr {
            ident: None,
            function: Box::new(self.handle_function(node)),
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

    fn handle_function(&mut self, node: &ir::FunctionDefinition) -> swc::Function {
        let params = node
            .params
            .iter()
            .map(|p| swc::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: swc::Pat::Ident(ident_from_str(&p.as_name()).into()),
            })
            .collect();

        swc::Function {
            params,
            body: Some(self.block_to_swc_stmt(&node.body)),
            ..Default::default()
        }
    }
}
