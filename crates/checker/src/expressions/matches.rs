use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_types::{store::TypeStore, types};

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
        let arms = self.visit_match_arms(&scrutinee, node.arms)?;
        let scrutinee = scrutinee?;

        let arm_patterns = arms.iter().map(|a| &a.0).collect::<Vec<_>>();
        self.check_exhaustiveness(arm_patterns, scrutinee.loc());

        let ty = self.check_match_type(&arms);

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
        let mut self_ = self.with_local_scope();
        arms?
            .into_iter()
            .map(|arm| self_.visit_match_arm(scrutinee.as_ref(), arm))
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

    /// Check if a list of patterns cover all possible cases
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

    fn check_match_type(&mut self, arms: &[(ir::Pattern, ir::Expression)]) -> types::TypeId {
        let mut valued_arms = arms.into_iter().filter(|a| !a.1.diverges());
        let ty = valued_arms.next().map_or(TypeStore::UNKNOWN, |a| a.1.ty());
        valued_arms.for_each(|a| self.check_assigned_type(ty, a.1.ty(), false, a.1.loc()));
        ty
    }
}

#[cfg(test)]
mod tests {
    use tine_ast::MatchArm;
    use tine_common::locations::Location;
    use tine_types::types::Type;

    use super::*;

    fn mock_bool() -> ast::BooleanLiteral {
        ast::BooleanLiteral::new(true, Location::dummy()).into()
    }

    fn true_lit() -> ast::BooleanLiteral {
        ast::BooleanLiteral::new(true, Location::dummy()).into()
    }
    fn false_lit() -> ast::BooleanLiteral {
        ast::BooleanLiteral::new(false, Location::dummy()).into()
    }

    fn int_lit() -> ast::IntLiteral {
        ast::IntLiteral::new(0, Location::dummy()).into()
    }

    fn block_with_return() -> ast::BlockExpression {
        ast::BlockExpression::from(ast::Statement::Return(ast::ReturnStatement {
            loc: Location::dummy(),
            value: None,
        }))
    }

    #[test]
    fn test_visit_match_expression_consistent_types() {
        let mut checker = TypeChecker::new();
        let array_expression = ast::MatchExpression {
            scrutinee: Some(Box::new(mock_bool().into())),
            arms: Some(vec![
                MatchArm::valid(ast::Pattern::Literal(true_lit().into()), true_lit().into()),
                MatchArm::valid(
                    ast::Pattern::Literal(false_lit().into()),
                    false_lit().into(),
                ),
            ]),
            loc: Location::dummy(),
        };

        let result = checker
            .visit_match_expression(array_expression)
            .expect("expected a match expression");
        let result = checker.resolve(result.ty);
        assert_eq!(result, Type::Boolean);
        assert!(
            checker.diagnostics.is_empty(),
            "expected no diagnostics, found {:?}",
            checker.diagnostics
        );
    }

    #[test]
    fn test_visit_match_expression_mixed_types() {
        let mut checker = TypeChecker::new();
        let array_expression = ast::MatchExpression {
            scrutinee: Some(Box::new(mock_bool().into())),
            arms: Some(vec![
                MatchArm::valid(ast::Pattern::Literal(true_lit().into()), true_lit().into()),
                MatchArm::valid(ast::Pattern::Literal(false_lit().into()), int_lit().into()),
            ]),
            loc: Location::dummy(),
        };

        let result = checker
            .visit_match_expression(array_expression)
            .expect("expected a match expression");
        let result = checker.resolve(result.ty);
        assert_eq!(result, Type::Boolean);
        assert_eq!(checker.diagnostics.len(), 1);
    }

    #[test]
    fn test_visit_match_expression_diverging() {
        let mut checker = TypeChecker::new();
        let array_expression = ast::MatchExpression {
            scrutinee: Some(Box::new(mock_bool().into())),
            arms: Some(vec![
                MatchArm::valid(ast::Pattern::Literal(true_lit().into()), true_lit().into()),
                MatchArm::valid(
                    ast::Pattern::Literal(false_lit().into()),
                    block_with_return().into(),
                ),
            ]),
            loc: Location::dummy(),
        };

        let result = checker
            .visit_match_expression(array_expression)
            .expect("expected a match expression");
        let result = checker.resolve(result.ty);
        assert_eq!(result, Type::Boolean);
        assert!(
            checker.diagnostics.is_empty(),
            "expected no diagnostics, found {:?}",
            checker.diagnostics
        );
    }
}
