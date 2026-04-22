use crate::{
    ast,
    diagnostics::DiagnosticKind,
    ir,
    type_checker::{
        analysis_context::type_store::TypeStore, utils::make_simple_declaration, TypeChecker,
    },
    types::{OptionType, TypeId},
};

impl TypeChecker<'_> {
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
                type_name: self.session.display_type(condition_type),
            };
            self.error(error, condition.loc());
        }
        Some(condition)
    }

    pub fn visit_if_decl_expression(
        &mut self,
        node: ast::IfPatExpression,
    ) -> Option<ir::IfExpression> {
        let lowered = self.lower_if_decl(node);
        self.visit_if_expression(lowered)
    }

    fn lower_if_decl(&mut self, node: ast::IfPatExpression) -> ast::IfExpression {
        let (Some(pattern), Some(scrutinee)) = (node.pattern, node.scrutinee) else {
            return ast::IfExpression {
                loc: node.loc,
                condition: None,
                consequent: node.consequent,
                alternate: node.alternate,
            };
        };

        let desugared = self.desugar_pattern(pattern, *scrutinee, false);
        let bindings = desugared
            .bindings
            .into_iter()
            .map(|(identifier, value)| make_simple_declaration(identifier, value).into())
            .collect();
        let body = node.consequent.map(|mut body| {
            body.statements = vec![bindings, body.statements].concat();
            body
        });

        ast::IfExpression {
            loc: node.loc,
            condition: desugared.test.map(Box::new),
            consequent: body,
            alternate: node.alternate,
        }
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
                    expected: self.session.display_type(expected),
                    got: self.session.display_type(alternate.ty),
                };
                self.error(error, alternate.loc);
            }
        }
        Some(alternate)
    }
}

#[cfg(test)]
mod tests {
    use crate::{type_checker::test_utils::MockLoader, types, Location, Session};

    use super::*;

    fn visit_if_expression(node: ast::IfExpression) -> (TypeId, TypeChecker<'static>) {
        let session = Session::new(Box::new(MockLoader));
        let mut checker = TypeChecker::new(Box::leak(Box::new(session)), 0);
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
        let (ty, checker) = visit_if_expression(node);
        assert_eq!(checker.diagnostics.len(), 0);
        assert_eq!(
            ty,
            checker.session.intern(types::Type::Option(OptionType {
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
            checker.diagnostics[0].kind,
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
            checker.diagnostics[0].kind,
            DiagnosticKind::MismatchedBranchTypes { .. }
        ))
    }
}
