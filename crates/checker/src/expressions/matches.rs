use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};

use crate::{
    exhaustiveness::{display_pattern, UsefulnessChecker},
    patterns::{lower_pattern, Pattern},
    TypeChecker,
};

impl TypeChecker {
    pub fn visit_match_expression(&mut self, node: ast::MatchExpression) -> Option<ir::Expression> {
        let scrutinee = node.scrutinee.and_then(|s| self.visit_expression(*s));
        let arms = self.with_scope(|self_| self_.visit_match_arms(&scrutinee, node.arms))?;
        let scrutinee = scrutinee?;

        self.check_match_arms(
            arms.iter().map(|a| &a.0).collect::<Vec<_>>(),
            scrutinee.loc(),
        );

        arms.into_iter()
            .rev()
            .fold(None, |alternate, (pattern, expr)| {
                Some(match_arm_to_if_else(pattern, expr, alternate))
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
        self.with_scope(|self_| {
            let pattern =
                scrutinee.and_then(|s| self_.visit_pattern(*arm.pattern?, s, true, false));
            let expression = arm.expression.and_then(|e| self_.visit_expression(*e));
            Some((pattern?, expression?))
        })
    }

    fn check_match_arms(&mut self, patterns: Vec<&ir::Pattern>, scrutinee_loc: Location) -> bool {
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
            self.error(diag, scrutinee_loc);
            return false;
        }
        true
    }
}

fn match_arm_to_if_else(
    pattern: Pattern,
    body: ir::Expression,
    alternate: Option<ir::Expression>,
) -> ir::Expression {
    let ty = body.ty();
    let lowered = lower_pattern(pattern, body.clone());
    let decls: Vec<ir::Statement> = lowered.decls.into_iter().map(Into::into).collect();
    let mut block: ir::Block = body.into();
    block.statements.splice(0..0, decls);

    match alternate {
        Some(alternate) => ir::Expression::If(ir::IfExpression {
            loc: block.loc,
            condition: Box::new(lowered.test.unwrap_or(ir::Expression::BooleanLiteral(
                ir::BooleanLiteral {
                    loc: block.loc,
                    value: true,
                },
            ))),
            consequent: block,
            alternate: Some(alternate.into()),
            ty,
        }),
        None => block.into(),
    }
}
