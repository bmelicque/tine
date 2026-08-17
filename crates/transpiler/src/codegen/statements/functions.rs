use crate::codegen::{
    utils::{ident_from_str, member},
    CodeGenerator,
};
use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_common::locations::Location;
use tine_ir as ir;
use tine_symbols::symbols::*;

impl CodeGenerator<'_, '_> {
    pub fn handle_function_definition(&mut self, node: ir::FunctionDefinition) -> swc::Stmt {
        match node.name.1 {
            ir::FunctionName::Function(f) => self.handle_regular_function(node, f),
            ir::FunctionName::StaticMethod(m) => self.handle_static_method(node, m),
        }
    }

    fn handle_regular_function(
        &mut self,
        node: ir::FunctionDefinition,
        name: FunctionSymbolId,
    ) -> swc::Stmt {
        let name = &self.symbols.get(name).name;
        swc::Stmt::Decl(swc::Decl::Fn(swc::FnDecl {
            ident: ident_from_str(name),
            declare: false,
            function: Box::new(self.handle_function(node.params, node.body)),
        }))
    }

    pub fn handle_static_method(
        &mut self,
        node: ir::FunctionDefinition,
        name: MethodSymbolId,
    ) -> swc::Stmt {
        let method = self.symbols.get(name);
        let constructor = self.generate_constructor_name(method.owner.into(), &method.owner_args);

        let name = self.symbol_name(name);

        let left =
            swc::AssignTarget::Simple(swc::SimpleAssignTarget::Member(member(constructor, name)));

        let right = Box::new(swc::Expr::Fn(swc::FnExpr {
            ident: None,
            function: Box::new(self.handle_function(node.params, node.body)),
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

    pub fn handle_function(
        &mut self,
        params: Vec<(Location, VariableSymbolId)>,
        body: ir::Block,
    ) -> swc::Function {
        let params = params
            .iter()
            .map(|p| swc::Param {
                span: DUMMY_SP,
                decorators: vec![],
                pat: swc::Pat::Ident(ident_from_str(&self.symbols.get(p.1).name).into()),
            })
            .collect();

        swc::Function {
            params,
            body: Some(self.handle_block_stmt(body)),
            ..Default::default()
        }
    }

    pub fn handle_method_definition(&mut self, node: ir::MethodDefinition) -> swc::Stmt {
        let method = self.symbols.get(node.name.1);
        let constructor = self.generate_constructor_name(method.owner.into(), &method.owner_args);
        let left = member(member(constructor, "prototype").into(), &method.name).into();
        let right = self.handle_function(node.params, node.body).into();

        swc::Stmt::Expr(swc::ExprStmt {
            span: DUMMY_SP,
            expr: Box::new(swc::Expr::Assign(swc::AssignExpr {
                left,
                right: right,
                ..Default::default()
            })),
        })
    }
}
