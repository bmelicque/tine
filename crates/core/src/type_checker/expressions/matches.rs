use crate::{
    ast, ir,
    type_checker::{
        patterns::{display_pattern, lower_pattern, Pattern},
        TypeChecker,
    },
    types, DiagnosticKind, Location,
};

impl TypeChecker {
    pub fn visit_match_expression(&mut self, node: ast::MatchExpression) -> Option<ir::Expression> {
        let scrutinee = node.scrutinee.and_then(|s| self.visit_expression(*s));
        let arms = node
            .arms?
            .into_iter()
            .map(|arm| {
                let expression = arm.expression.and_then(|e| self.visit_expression(*e))?;

                let pattern = scrutinee
                    .as_ref()
                    .and_then(|s| self.visit_pattern(*arm.pattern?, s, true));

                Some((pattern?, expression))
            })
            .collect::<Option<Vec<_>>>()?;

        let scrutinee = scrutinee?;

        self.check_match_arms(
            arms.iter().map(|a| &a.0).collect::<Vec<_>>(),
            scrutinee.loc(),
        );

        // TODO: check exhaustiveness. If not exhaustive, return `None`
        arms.into_iter()
            .rev()
            .fold(None, |alternate, (pattern, expr)| {
                Some(match_arm_to_if_else(
                    pattern,
                    expr,
                    alternate,
                    scrutinee.ty(),
                ))
            })
    }

    fn check_match_arms(&mut self, patterns: Vec<&Pattern>, scrutinee_loc: Location) -> bool {
        let matrix = patterns
            .into_iter()
            .map(|pat| vec![pat])
            .collect::<Vec<_>>();
        let u = self.usefulness(&matrix, &vec![&Pattern::Wildcard]);
        if !u.is_empty() {
            let missing = u
                .into_iter()
                .map(|r| display_pattern(&r[0], &self.symbols))
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
    ty: types::TypeId,
) -> ir::Expression {
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
