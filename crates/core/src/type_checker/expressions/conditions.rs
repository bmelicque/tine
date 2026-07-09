use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::{patterns::lower_pattern, type_store::TypeStore, TypeChecker},
    types::{self, OptionType, TypeId},
};

impl TypeChecker {
    pub fn visit_if_expression(&mut self, node: ast::IfExpression) -> Option<ir::IfExpression> {
        let condition = node.condition.and_then(|c| self.visit_condition(*c));
        let consequent = node
            .consequent
            .map(|c| self.with_scope(|s| s.visit_block_expression(c)));
        let ty = consequent.as_ref().map(|c| c.ty);

        let (alternate, ty) = if let Some(alternate) = node.alternate {
            let Some(alternate) = self.visit_alternate(*alternate, ty) else {
                return None;
            };
            (Some(alternate), ty?)
        } else {
            (None, self.intern(OptionType { some: ty? }))
        };

        let Some(condition) = condition else {
            return None;
        };
        let Some(consequent) = consequent else {
            return None;
        };

        Some(ir::IfExpression {
            loc: node.loc,
            condition: Box::new(condition),
            consequent,
            alternate,
            ty,
        })
    }

    pub fn visit_condition(&mut self, node: ast::Expression) -> Option<ir::Expression> {
        let Some(condition) = self.visit_expression(node) else {
            return None;
        };
        let condition_type = condition.ty();
        if condition_type != TypeStore::BOOLEAN && condition_type != TypeStore::UNKNOWN {
            let error = DiagnosticKind::InvalidCondition {
                type_name: self.types.display(condition_type),
            };
            self.error(error, condition.loc());
        }
        Some(condition)
    }

    pub fn visit_if_decl_expression(
        &mut self,
        node: ast::IfPatExpression,
    ) -> Option<ir::IfExpression> {
        let (Some(pattern), Some(scrutinee)) = (node.pattern, node.scrutinee) else {
            let node = ast::IfExpression {
                loc: node.loc,
                condition: None,
                consequent: node.consequent,
                alternate: node.alternate,
            };
            return self.visit_if_expression(node);
        };
        let value = self.visit_expression(*scrutinee)?;
        let pattern_loc = pattern.loc();

        let (test, consequent) = self.with_scope(|self_| {
            let pattern = self_.visit_pattern(pattern, &value, true, false)?;
            let lowered = lower_pattern(pattern, value);
            let mut consequent = node.consequent.map(|c| self_.visit_block_expression(c));
            if let Some(ref mut consequent) = consequent {
                let decls: Vec<ir::Statement> = lowered.decls.into_iter().map(Into::into).collect();
                consequent.statements.splice(0..0, decls);
            }
            Some((lowered.test, consequent))
        })?;

        let alternate = node
            .alternate
            .and_then(|a| self.visit_alternate(*a, consequent.as_ref().map(|b| b.ty)));

        let Some(test) = test else {
            self.error(DiagnosticKind::RefutablePatternExpected, pattern_loc);
            return None;
        };

        let ty = self.get_if_type(
            consequent.as_ref().map_or(TypeStore::UNKNOWN, |c| c.ty),
            &alternate,
        );

        Some(ir::IfExpression {
            loc: node.loc,
            condition: Box::new(test),
            consequent: consequent?,
            alternate,
            ty,
        })
    }

    fn visit_alternate(
        &mut self,
        alternate: ast::Alternate,
        expected: Option<TypeId>,
    ) -> Option<ir::Block> {
        let alternate = match alternate {
            ast::Alternate::Block(b) => Some(self.visit_block_expression(b)),
            ast::Alternate::If(i) => self
                .visit_if_expression(i)
                .map(|e| ir::Expression::If(e).into()),
            ast::Alternate::IfDecl(i) => self
                .visit_if_decl_expression(i)
                .map(|e| ir::Expression::If(e).into()),
        }?;
        if let Some(expected) = expected {
            if !self.can_be_assigned_to(alternate.ty, expected) {
                let error = DiagnosticKind::MismatchedBranchTypes {
                    expected: self.types.display(expected),
                    got: self.types.display(alternate.ty),
                };
                self.error(error, alternate.loc);
            }
        }
        Some(alternate)
    }

    fn get_if_type(
        &mut self,
        consequent: types::TypeId,
        alternate: &Option<ir::Block>,
    ) -> types::TypeId {
        let Some(alternate) = alternate else {
            return self.intern(types::OptionType { some: consequent });
        };

        match self.resolve(alternate.ty) {
            types::Type::Option(_) => self.intern(types::OptionType { some: consequent }),
            _ => consequent,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{types, Location};

    use super::*;

    fn visit_if_expression(node: ast::IfExpression) -> (TypeId, TypeChecker) {
        let mut checker = TypeChecker::new();
        let ty = checker
            .visit_if_expression(node)
            .map_or(TypeStore::UNKNOWN, |e| e.ty);
        (ty, checker)
    }

    fn mock_condition() -> ast::Expression {
        ast::Expression::BooleanLiteral(ast::BooleanLiteral {
            loc: Location::dummy(),
            value: true,
        })
    }
    fn mock_bad_condition() -> ast::Expression {
        ast::Expression::IntLiteral(ast::IntLiteral {
            loc: Location::dummy(),
            value: 0,
        })
    }

    fn int_block_expression() -> ast::BlockExpression {
        ast::BlockExpression {
            loc: Location::dummy(),
            statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                expression: Box::new(ast::Expression::IntLiteral(ast::IntLiteral {
                    loc: Location::dummy(),
                    value: 1,
                })),
            })],
        }
    }
    fn bool_block_expression() -> ast::BlockExpression {
        ast::BlockExpression {
            loc: Location::dummy(),
            statements: vec![ast::Statement::Expression(ast::ExpressionStatement {
                expression: Box::new(ast::Expression::BooleanLiteral(ast::BooleanLiteral {
                    loc: Location::dummy(),
                    value: true,
                })),
            })],
        }
    }

    #[test]
    fn test_visit_if_expression() {
        let node = ast::IfExpression {
            loc: Location::dummy(),
            condition: Some(Box::new(mock_condition())),
            consequent: Some(int_block_expression()),
            alternate: None,
        };
        let (ty, mut checker) = visit_if_expression(node);
        assert_eq!(checker.diagnostics.len(), 0);
        assert_eq!(
            ty,
            checker.types.add(types::Type::Option(OptionType {
                some: TypeStore::INTEGER
            }))
        );
    }

    #[test]
    fn test_visit_if_expression_with_bad_condition() {
        let node = ast::IfExpression {
            loc: Location::dummy(),
            condition: Some(Box::new(mock_bad_condition())),
            consequent: Some(int_block_expression()),
            alternate: None,
        };
        let (_, checker) = visit_if_expression(node);
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[&0][0].kind,
            DiagnosticKind::InvalidCondition { .. }
        ));
    }

    #[test]
    fn test_visit_if_expression_with_alternate() {
        let node = ast::IfExpression {
            loc: Location::dummy(),
            condition: Some(Box::new(mock_condition())),
            consequent: Some(int_block_expression()),
            alternate: Some(Box::new(int_block_expression().into())),
        };
        let (ty, checker) = visit_if_expression(node);
        assert_eq!(checker.diagnostics.len(), 0);
        assert_eq!(ty, TypeStore::INTEGER);
    }

    #[test]
    fn test_visit_if_expression_with_alternate_mismatch() {
        let node = ast::IfExpression {
            loc: Location::dummy(),
            condition: Some(Box::new(mock_condition())),
            consequent: Some(int_block_expression()),
            alternate: Some(Box::new(bool_block_expression().into())),
        };
        let (_, checker) = visit_if_expression(node);
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[&0][0].kind,
            DiagnosticKind::MismatchedBranchTypes { .. }
        ))
    }
}
