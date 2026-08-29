use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_types::store::TypeStore;

use crate::{
    exhaustiveness::{display_pattern, UsefulnessChecker},
    TypeChecker,
};

impl TypeChecker {
    pub fn visit_match_expression(
        &mut self,
        node: ast::MatchExpression,
    ) -> Option<ir::MatchExpression> {
        let scrutinee = node.scrutinee.and_then(|s| self.visit_expression(*s));
        let arms = self.with_scope(|self_| self_.visit_match_arms(&scrutinee, node.arms))?;
        let scrutinee = scrutinee?;

        self.check_exhaustiveness(
            arms.iter().map(|a| &a.0).collect::<Vec<_>>(),
            scrutinee.loc(),
        );

        let ty = arms.first().map_or(TypeStore::UNKNOWN, |a| a.1.ty());
        arms.iter()
            .skip(1)
            .for_each(|a| self.check_assigned_type(ty, a.1.ty(), false, a.1.loc()));

        Some(ir::MatchExpression {
            ty,
            loc: node.loc,
            scrutinee: Box::new(scrutinee),
            arms,
        })
    }

    fn visit_match_arms(
        &mut self,
        scrutinee: &Option<ir::Expression>,
        arms: Option<Vec<ast::MatchArm>>,
    ) -> Option<Vec<(ir::Pattern, ir::Expression)>> {
        arms?
            .into_iter()
            .map(|arm| self.visit_match_arm(scrutinee.as_ref(), arm))
            .collect::<Option<Vec<_>>>()
    }

    fn visit_match_arm(
        &mut self,
        scrutinee: Option<&ir::Expression>,
        arm: ast::MatchArm,
    ) -> Option<(ir::Pattern, ir::Expression)> {
        let mut self_ = self.with_local_scope();
        let pattern = scrutinee.and_then(|s| self_.visit_pattern(*arm.pattern?, s, s.ty(), false));
        let expression = arm.expression.and_then(|e| self_.visit_expression(*e));
        Some((pattern?, expression?))
    }

    pub fn check_exhaustiveness(&mut self, patterns: Vec<&ir::Pattern>, loc: Location) -> bool {
        let matrix = patterns
            .into_iter()
            .map(|pat| vec![pat])
            .collect::<Vec<_>>();
        let mut uc = UsefulnessChecker::new(self);
        let u = self.usefulness(&mut uc, &matrix, &vec![ir::Pattern::wildcard()]);
        if !u.is_empty() {
            let missing = u
                .into_iter()
                .map(|r| display_pattern(&mut uc, &r[0]))
                .collect();
            let diag = DiagnosticKind::NonExhaustiveMatch { missing };
            self.error(diag, loc);
            return false;
        }
        true
    }
}
