use crate::{
    ast::{self, Pattern},
    ir::{self, root_identifier},
    type_checker::{patterns::lower_pattern, TypeChecker},
    types::{self, Type, TypeId},
    DiagnosticKind, TypeStore,
};

impl TypeChecker<'_> {
    pub fn visit_assignment(&mut self, node: ast::Assignment) -> Vec<ir::Statement> {
        let value = node.value.and_then(|v| self.visit_expression(v));
        let Some(pattern) = node.pattern else {
            return vec![];
        };
        let value_type = value.as_ref().map_or(TypeStore::UNKNOWN, |v| v.ty());
        let assignee = self.visit_assignee(pattern, value_type);
        let Some(value) = value else { return vec![] };
        match assignee {
            Ok(Some(assignee)) => {
                vec![ir::Statement::Assignment(ir::Assignment {
                    loc: node.loc,
                    pattern: assignee,
                    value,
                })]
            }
            Ok(None) => vec![],
            Err(pattern) => {
                let mut stmts = Vec::new();
                let loc = pattern.loc();

                let id: ir::Expression = self.make_temp_variable(loc, &value).into();
                stmts.push(ir::Statement::Assignment(ir::Assignment {
                    loc,
                    pattern: id.clone(),
                    value,
                }));

                let pattern = self.visit_pattern(pattern, &id, false);
                let Some(pattern) = pattern else {
                    return vec![];
                };
                let lowered = lower_pattern(pattern, id);
                if lowered.test.is_some() {
                    self.error(DiagnosticKind::IrrefutablePatternExpected, loc);
                    return vec![];
                }
                let decls = lowered
                    .decls
                    .into_iter()
                    .map(Into::into)
                    .collect::<Vec<_>>();
                stmts.extend(decls);
                stmts
            }
        }
    }

    fn visit_assignee(
        &mut self,
        assignee: ast::Assignee,
        ty: types::TypeId,
    ) -> Result<Option<ir::Expression>, Pattern> {
        match assignee {
            ast::Assignee::Member(m) => Ok(self.visit_expr_assignee(m, ty)),
            ast::Assignee::Indirection(i) => Ok(self.visit_indirect_assignee(i, ty)),
            ast::Assignee::Pattern(ast::Pattern::Identifier(i)) => {
                Ok(self.visit_identifier_assignee(i, ty))
            }
            ast::Assignee::Pattern(pattern) => Err(pattern),
        }
    }

    /// Visit an assignee which is a pattern
    fn visit_identifier_assignee(
        &mut self,
        pattern: ast::IdentifierPattern,
        against: TypeId,
    ) -> Option<ir::Expression> {
        let ast::IdentifierPattern(identifier) = pattern;
        let identifier = self.visit_identifier(identifier)?;
        let handle = self.get_handle(identifier.symbol.clone())?;
        if !handle.borrow().is_mutable() {
            let error = DiagnosticKind::AssignmentToConstant {
                name: identifier.as_name(),
            };
            self.error(error, identifier.loc);
        }
        self.check_assigned_type(handle.borrow().get_type(), against, false, identifier.loc);
        Some(identifier.into())
    }

    fn visit_expr_assignee(
        &mut self,
        expr: ast::MemberExpression,
        against: TypeId,
    ) -> Option<ir::Expression> {
        let expression = self.visit_member_expression(expr)?.into();
        if let Some(root) = root_identifier(&expression) {
            if let Some(handle) = self.session.get_handle(root.symbol.clone()) {
                // visit expression adds a read that need to be converted to write
                handle.read_to_write(root.loc);
            }
            if !root.symbol.borrow().is_mutable() {
                let error = DiagnosticKind::AssignmentToConstant {
                    name: root.as_name(),
                };
                self.error(error, expression.loc());
            }
        }
        self.check_assigned_type(against, expression.ty(), false, expression.loc());
        Some(expression)
    }

    fn visit_indirect_assignee(
        &mut self,
        node: ast::IndirectionAssignee,
        against: TypeId,
    ) -> Option<ir::Expression> {
        let name = node.identifier.as_str();
        let Some(info) = self.lookup_mut(&name) else {
            let error = DiagnosticKind::CannotFindName {
                name: name.to_string(),
            };
            self.error(error, node.identifier.loc);
            return None;
        };
        info.write(node.identifier.loc);
        let ty = info.borrow().get_type();
        let ty = match self.resolve(ty).clone() {
            Type::Signal(t) => {
                self.check_assigned_type(t.inner, against, false, node.loc);
                t.inner
            }
            Type::Listener(t) => {
                self.check_assigned_type(t.inner, against, false, node.loc);
                t.inner
            }
            _ => {
                let error = DiagnosticKind::NotDereferenceable {
                    type_name: self.session.display_type(ty),
                };
                self.error(error, node.loc);
                return None;
            }
        };

        Some(ir::Expression::Unary(ir::UnaryExpression {
            loc: node.loc,
            operator: ir::UnaryOperator::Star,
            operand: Box::new(ir::Expression::Identifier(ir::Identifier {
                loc: node.loc,
                symbol: info.readonly(),
            })),
            ty,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast,
        type_checker::{test_utils::MockLoader, TypeChecker},
        DiagnosticKind, Location, Session, SymbolData, SymbolKind, TypeStore,
    };

    fn make_type_checker() -> TypeChecker<'static> {
        let session = Session::new(Box::new(MockLoader));
        TypeChecker::new(Box::leak(Box::new(session)), 0)
    }

    fn dummy_assignment() -> ast::Assignment {
        ast::Assignment {
            loc: Location::dummy(),
            pattern: Some(ast::Assignee::Pattern(ast::Pattern::Identifier(
                ast::IdentifierPattern(ast::Identifier {
                    loc: Location::dummy(),
                    text: "a".to_string(),
                }),
            ))),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                loc: Location::dummy(),
                value: 1,
            })),
        }
    }

    #[test]
    fn visit_assignment_simple() {
        let mut checker = make_type_checker();
        checker.ctx.register_symbol(SymbolData {
            name: "a".to_string(),
            ty: TypeStore::INTEGER,
            kind: SymbolKind::Value { mutable: true },
            ..Default::default()
        });

        checker.visit_assignment(dummy_assignment());
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn visit_assignment_to_constant() {
        let mut checker = make_type_checker();
        checker.ctx.register_symbol(SymbolData {
            name: "a".to_string(),
            ty: TypeStore::INTEGER,
            kind: SymbolKind::Value { mutable: false },
            ..Default::default()
        });
        checker.visit_assignment(dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[0].kind,
            DiagnosticKind::AssignmentToConstant { .. }
        ))
    }

    #[test]
    fn visit_assignment_bad_type() {
        let mut checker = make_type_checker();
        checker.ctx.register_symbol(SymbolData {
            name: "a".to_string(),
            ty: TypeStore::FLOAT,
            kind: SymbolKind::Value { mutable: true },
            ..Default::default()
        });
        checker.visit_assignment(dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[0].kind,
            DiagnosticKind::WrongType { .. }
        ));
    }

    #[test]
    fn visit_assignment_unknown_variable() {
        let mut checker = make_type_checker();
        checker.visit_assignment(dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[0].kind,
            DiagnosticKind::CannotFindName { .. }
        ));
    }
}
