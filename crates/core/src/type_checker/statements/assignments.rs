use crate::{
    ast,
    ir::{self, root_identifier},
    type_checker::{utils::make_tmp_declaration, TypeChecker},
    types::{Type, TypeId},
    DiagnosticKind, TypeStore,
};

impl TypeChecker<'_> {
    pub fn visit_assignment(&mut self, node: ast::Assignment) -> Vec<ir::Statement> {
        self.desugar_assignment(node)
            .into_iter()
            .flat_map(|a| match a {
                ast::Statement::Assignment(a) => self
                    .visit_simple_assignment(a)
                    .map_or(vec![], |a| vec![a.into()]),
                _ => self.visit_statement(a),
            })
            .collect()
    }

    /// Lower an assignment that uses pattern matching into several simple assignments.
    /// Also make sure that pattern is irrefutable.
    fn desugar_assignment(&mut self, node: ast::Assignment) -> Vec<ast::Statement> {
        let Some(ast::Assignee::Pattern(pattern)) = &node.pattern else {
            return vec![node.into()];
        };
        let (tmp_decl, desugared) = match pattern {
            ast::Pattern::Constructor(_) | ast::Pattern::Tuple(_) => {
                let Some(value) = node.value else {
                    return vec![];
                };
                let tmp_decl = make_tmp_declaration(value);
                let Some(ast::Pattern::Identifier(ast::IdentifierPattern(ident))) =
                    tmp_decl.pattern.clone()
                else {
                    panic!()
                };
                let loc = pattern.loc();
                let desugared = self.desugar_pattern(pattern.to_owned(), ident.into(), true);
                if desugared.test.is_some() {
                    self.error(DiagnosticKind::IrrefutablePatternExpected, loc);
                }
                (tmp_decl, desugared)
            }
            _ => return vec![node.into()],
        };

        let mut statements = desugared
            .bindings
            .into_iter()
            .map(|(identifier, value)| {
                ast::Statement::Assignment(ast::Assignment {
                    loc: node.loc,
                    pattern: Some(ast::Assignee::Pattern(identifier.into())),
                    value: Some(value),
                })
            })
            .collect::<Vec<_>>();
        statements.insert(0, tmp_decl.into());
        statements
    }

    fn visit_simple_assignment(&mut self, node: ast::Assignment) -> Option<ir::Assignment> {
        let value = node.value.and_then(|v| self.visit_expression(v));
        let value_type = value.as_ref().map_or(TypeStore::UNKNOWN, |v| v.ty());
        let assignee = node
            .pattern
            .and_then(|p| self.visit_assignee(p, value_type));
        Some(ir::Assignment {
            loc: node.loc,
            pattern: assignee?,
            value: value?,
        })
    }

    /// Visit an assignee (i.e. the lhs of an assignment)
    fn visit_assignee(
        &mut self,
        assignee: ast::Assignee,
        against: TypeId,
    ) -> Option<ir::Expression> {
        match assignee {
            ast::Assignee::Member(expr) => self.visit_expr_assignee(expr, against),
            ast::Assignee::Indirection(expr) => self.visit_indirect_assignee(expr, against),
            ast::Assignee::Pattern(pat) => self.visit_pattern_assignee(pat, against),
        }
    }

    /// Visit an assignee which is a pattern
    fn visit_pattern_assignee(
        &mut self,
        pattern: ast::Pattern,
        against: TypeId,
    ) -> Option<ir::Expression> {
        let ast::Pattern::Identifier(ast::IdentifierPattern(identifier)) = pattern else {
            panic!("this should only be called with identifiers")
        };
        let identifier = self.visit_identifier(identifier)?;
        let handle = self.session.get_handle(identifier.symbol.clone())?;
        if !handle.borrow().is_mutable() {
            let error = DiagnosticKind::AssignmentToConstant {
                name: identifier.as_name(),
            };
            self.error(error, identifier.loc);
        }
        self.check_assigned_type(handle.borrow().get_type(), against, identifier.loc);
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
        self.check_assigned_type(against, expression.ty(), expression.loc());
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
                self.check_assigned_type(t.inner, against, node.loc);
                t.inner
            }
            Type::Listener(t) => {
                self.check_assigned_type(t.inner, against, node.loc);
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
