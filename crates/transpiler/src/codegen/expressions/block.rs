use swc_common::DUMMY_SP;
use tine_core::ir;

use crate::codegen::{
    expressions::{
        utils::{assign_block_last_expression, ident_to_declaration},
        ExpressionResult,
    },
    utils::{can_block_be_inlined, undefined},
    CodeGenerator,
};

use swc_ecma_ast as swc;

impl CodeGenerator<'_> {
    pub fn handle_block(&mut self, node: &ir::Block) -> ExpressionResult {
        if node.statements.len() == 0 {
            ExpressionResult {
                prelim_stmts: vec![],
                expr: undefined(),
            }
        } else if can_block_be_inlined(node) {
            self.inlined_block(node).into()
        } else {
            self.block_to_extracted(node)
        }
    }

    pub fn inlined_block(&mut self, node: &ir::Block) -> swc::Expr {
        if node.statements.len() == 1 {
            let ir::Statement::Expression(expr) = &node.statements[0] else {
                panic!()
            };
            let result = self.handle_expression(expr);
            debug_assert_eq!(result.prelim_stmts.len(), 0);
            return result.expr;
        }

        let exprs = node
            .statements
            .iter()
            .map(|stmt| match stmt {
                ir::Statement::Assignment(a) => {
                    Box::new(self.handle_assignment_as_expr(a).expr.into())
                }
                ir::Statement::Expression(expr) => {
                    let result = self.handle_expression(expr);
                    debug_assert_eq!(result.prelim_stmts.len(), 0);
                    Box::new(result.expr)
                }
                _ => panic!(),
            })
            .collect();

        swc::Expr::Seq(swc::SeqExpr {
            span: DUMMY_SP,
            exprs,
        })
    }

    fn block_to_extracted(&mut self, node: &ir::Block) -> ExpressionResult {
        let temp = self.get_temp_id();
        let decl = ident_to_declaration(temp.clone());
        let mut block = self.block_to_swc_stmt(node);
        assign_block_last_expression(&mut block, temp.clone());

        ExpressionResult {
            prelim_stmts: vec![decl, block.into()],
            expr: temp.into(),
        }
    }
}
