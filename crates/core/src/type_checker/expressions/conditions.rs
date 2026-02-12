use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::{analysis_context::type_store::TypeStore, patterns::TokenList, TypeChecker},
    types::{OptionType, Type, TypeId},
    SymbolData, SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_if_expression(&mut self, node: &ast::IfExpression) -> TypeId {
        if let Some(condition) = &node.condition {
            self.visit_condition(condition);
        }
        let ty = match &node.consequent {
            Some(consequent) => self.with_scope(|s| s.visit_block_expression(consequent)),
            None => TypeStore::UNKNOWN,
        };
        let ty = if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, ty);
            ty
        } else {
            self.intern(Type::Option(OptionType { some: ty }))
        };
        self.ctx.save_expression_type(node.loc, ty)
    }

    pub fn visit_condition(&mut self, node: &ast::Expression) {
        let condition = self.visit_expression(node);
        if condition != TypeStore::BOOLEAN && condition != TypeStore::UNKNOWN {
            let error = DiagnosticKind::InvalidCondition {
                type_name: self.session.display_type(condition),
            };
            self.error(error, node.loc());
        }
    }

    pub fn visit_if_decl_expression(&mut self, node: &ast::IfPatExpression) -> TypeId {
        if let Some(pattern) = &node.pattern {
            if !pattern.is_refutable() {
                self.error(DiagnosticKind::RefutablePatternExpected, pattern.loc());
            };
        }

        let ty = self.with_scope(|s| {
            let (inferred_type, dependencies) = match &node.scrutinee {
                Some(e) => s.with_dependencies(|s| s.visit_expression(e)),
                None => (TypeStore::UNKNOWN, vec![]),
            };
            let mut variables = TokenList::new();
            if let Some(pattern) = &node.pattern {
                s.match_pattern(pattern, inferred_type.clone(), &mut variables);
            }
            if let Some(consequent) = &node.consequent {
                for (name, ty) in variables.0 {
                    s.ctx.register_symbol(SymbolData {
                        name: name.as_str().into(),
                        ty,
                        kind: SymbolKind::constant(),
                        defined_at: name.loc,
                        dependencies: dependencies.clone(),
                        ..Default::default()
                    });
                    s.ctx.save_expression_type(name.loc, ty);
                }
                s.visit_block_expression(consequent)
            } else {
                TypeStore::UNKNOWN
            }
        });

        let ty = if let Some(ref alternate) = node.alternate {
            self.visit_alternate(alternate, ty);
            ty
        } else {
            self.intern(Type::Option(OptionType { some: ty }))
        };
        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_alternate(&mut self, alternate: &ast::Alternate, expected: TypeId) {
        let alt_ty = match alternate {
            ast::Alternate::Block(b) => self.visit_block_expression(b),
            ast::Alternate::If(i) => self.visit_if_expression(i),
            ast::Alternate::IfDecl(i) => self.visit_if_decl_expression(i),
        };
        if !self.can_be_assigned_to(alt_ty, expected) {
            let error = DiagnosticKind::MismatchedBranchTypes {
                expected: self.session.display_type(expected),
                got: self.session.display_type(alt_ty),
            };
            self.error(error, alternate.loc())
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Location, Session};

    use super::*;

    fn visit_if_expression(node: ast::IfExpression) -> (TypeId, TypeChecker<'static>) {
        let session = Session::new();
        let mut checker = TypeChecker::new(Box::leak(Box::new(session)), 0);
        let ty = checker.visit_if_expression(&node);
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
            checker.session.intern(Type::Option(OptionType {
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
