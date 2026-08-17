use swc_common::DUMMY_SP;
use swc_ecma_ast as swc;
use tine_ir as ir;

use crate::codegen::{
    statements::utils::{declare_const, declare_pat},
    utils::create_bool,
    CodeGenerator,
};

impl CodeGenerator<'_, '_> {
    pub fn handle_match_statement(&mut self, node: ir::MatchExpression) -> Vec<swc::Stmt> {
        let scrutinee = self.handle_expression(*node.scrutinee);
        let mut stmts = scrutinee.prelim_stmts;
        let scrutinee_id = self.get_temp_id();
        stmts.push(declare_const(scrutinee_id.sym.as_str(), scrutinee.expr).into());
        let arms = self.handle_match_arms(node.arms, scrutinee_id.into());
        stmts.push(arms.into());
        stmts
    }

    fn handle_match_arms(
        &mut self,
        arms: Vec<(ir::Pattern, ir::Expression)>,
        scrutinee: swc::Expr,
    ) -> swc::IfStmt {
        debug_assert!(!arms.is_empty());
        arms.into_iter()
            .rev()
            .map(|arm| self.handle_match_arm(arm, scrutinee.clone()))
            .reduce(|alt, mut stmt| {
                stmt.alt = Some(Box::new(alt.into()));
                stmt
            })
            .unwrap()
    }

    fn handle_match_arm(
        &mut self,
        arm: (ir::Pattern, ir::Expression),
        scrutinee: swc::Expr,
    ) -> swc::IfStmt {
        let result = self.handle_pattern(arm.0, scrutinee.clone());
        let test = result.test.unwrap_or(create_bool(true));
        let mut cons = self.handle_block_stmt(ir::Block::from(arm.1));
        if let Some(name) = result.decl {
            let decl = declare_pat(name, scrutinee);
            cons.stmts.insert(0, decl.into());
        }

        swc::IfStmt {
            span: DUMMY_SP,
            test: Box::new(test),
            cons: Box::new(cons.into()),
            alt: None,
        }
    }
}
