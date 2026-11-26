use tine_ast as ast;
use tine_ir::{self as ir, Typed};
use tine_types::{store::TypeStore, types};

use crate::TypeChecker;

impl TypeChecker {
    pub fn visit_block_expression(&mut self, node: ast::BlockExpression) -> ir::Block {
        let mut self_ = self.with_local_scope();
        let statements = node
            .statements
            .into_iter()
            .flat_map(|stmt| self_.visit_statement(stmt))
            .collect::<Vec<_>>();

        let ty = get_block_type(&statements);

        ir::Block {
            loc: node.loc,
            statements,
            ty,
        }
    }
}

fn get_block_type(statements: &[ir::Statement]) -> types::TypeId {
    statements
        .last()
        .map_or(TypeStore::UNIT, |stmt| match stmt {
            ir::Statement::Expression(e) => e.ty(),
            _ => TypeStore::UNIT,
        })
}
