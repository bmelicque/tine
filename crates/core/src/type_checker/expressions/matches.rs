use crate::{
    ast, ir,
    type_checker::{utils::make_simple_declaration, TypeChecker},
};

impl TypeChecker<'_> {
    pub fn visit_match_expression(
        &mut self,
        node: ast::MatchExpression,
    ) -> Option<ir::IfExpression> {
        // TODO check exhaustiveness

        let scrutinee = node.scrutinee.as_deref().cloned();
        let mut arms = node.arms?.into_iter().rev();
        let mut expr = self.desugar_match_arm(arms.next()?, &scrutinee);
        for arm in arms {
            let mut arm = self.desugar_match_arm(arm, &scrutinee);
            arm.alternate = Some(Box::new(ast::Alternate::If(expr)));
            expr = arm;
        }

        self.visit_if_expression(expr)
    }

    fn desugar_match_arm(
        &mut self,
        arm: ast::MatchArm,
        scrutinee: &Option<ast::Expression>,
    ) -> ast::IfExpression {
        let consequent = arm.expression.map(|e| match *e {
            ast::Expression::Block(b) => b,
            e => ast::BlockExpression {
                loc: e.loc(),
                statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                    expression: Box::new(e),
                })],
            },
        });

        let (Some(pattern), Some(scrutinee)) = (arm.pattern, scrutinee) else {
            return ast::IfExpression {
                loc: arm.loc,
                condition: None,
                consequent,
                alternate: None,
            };
        };

        let desugared = self.desugar_pattern(*pattern, scrutinee.clone(), false);
        let bindings = desugared
            .bindings
            .into_iter()
            .map(|(identifier, value)| make_simple_declaration(identifier, value).into())
            .collect();
        let body = consequent.map(|mut body| {
            body.statements = vec![bindings, body.statements].concat();
            body
        });

        ast::IfExpression {
            loc: arm.loc,
            condition: desugared.test.map(Box::new),
            consequent: body,
            alternate: None,
        }
    }
}
