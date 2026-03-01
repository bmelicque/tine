use crate::{
    ast,
    type_checker::{patterns::TokenList, TypeChecker},
    types::{Type, TypeId},
    DiagnosticKind,
};

impl TypeChecker<'_> {
    /// Visit an assignee (i.e. the lhs of an assignment)
    pub fn visit_assignee(&mut self, assignee: &ast::Assignee, against: TypeId) {
        match assignee {
            ast::Assignee::Member(expr) => self.visit_expr_assignee(expr, against),
            ast::Assignee::Indirection(expr) => self.visit_indirect_assignee(expr, against),
            ast::Assignee::Pattern(pat) => self.visit_pattern_assignee(pat, against),
        }
    }

    /// Visit an assignee which is a pattern
    fn visit_pattern_assignee(&mut self, pattern: &ast::Pattern, against: TypeId) {
        let mut variables = TokenList::new();
        self.match_pattern(pattern, against, &mut variables);
        for (name, ty) in variables.0 {
            let Some(info) = self.lookup_mut(name.as_str()) else {
                self.error(
                    DiagnosticKind::CannotFindName {
                        name: name.as_str().to_string(),
                    },
                    pattern.loc(),
                );
                continue;
            };
            info.write(name.loc);
            self.check_assigned_type(info.borrow().get_type(), ty, pattern.loc());
            if !info.borrow().is_mutable() {
                let error = DiagnosticKind::AssignmentToConstant {
                    name: name.as_str().to_string(),
                };
                self.error(error, pattern.loc());
            }
        }
    }

    fn visit_expr_assignee(&mut self, expr: &ast::MemberExpression, against: TypeId) {
        let ty = self.visit_expression_box_option(&expr.object);
        self.check_assigned_type(against, ty, expr.loc);
        let root = expr.root_expression();
        let Some(ast::Expression::Identifier(root)) = root else {
            if let Some(root) = &root {
                self.error(DiagnosticKind::InvalidRootAssignee, root.loc());
            }
            return;
        };
        let Some(info) = self.lookup_mut(root.as_str()) else {
            let error = DiagnosticKind::CannotFindName {
                name: root.as_str().to_string(),
            };
            self.error(error, root.loc);
            return;
        };
        // visit expression at the beginning of the current scope adds a read
        // so we need to remove it here
        info.read_to_write(root.loc);
        if !info.borrow().is_mutable() {
            let error = DiagnosticKind::AssignmentToConstant {
                name: info.borrow().name.clone(),
            };
            self.error(error, expr.loc);
        }
    }

    fn visit_indirect_assignee(&mut self, node: &ast::IndirectionAssignee, against: TypeId) {
        let name = node.identifier.as_str();
        let Some(info) = self.lookup_mut(&name) else {
            let error = DiagnosticKind::CannotFindName {
                name: name.to_string(),
            };
            self.error(error, node.identifier.loc);
            return;
        };
        info.write(node.identifier.loc);
        let ty = info.borrow().get_type();
        match self.resolve(ty).clone() {
            Type::Signal(t) => {
                self.check_assigned_type(t.inner, against, node.loc);
            }
            Type::Listener(t) => {
                self.check_assigned_type(t.inner, against, node.loc);
            }
            _ => {
                let error = DiagnosticKind::NotDereferenceable {
                    type_name: self.session.display_type(ty),
                };
                self.error(error, node.loc);
            }
        }
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

        checker.visit_assignment(&dummy_assignment());
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
        checker.visit_assignment(&dummy_assignment());
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
        checker.visit_assignment(&dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[0].kind,
            DiagnosticKind::WrongType { .. }
        ));
    }

    #[test]
    fn visit_assignment_unknown_variable() {
        let mut checker = make_type_checker();
        checker.visit_assignment(&dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[0].kind,
            DiagnosticKind::CannotFindName { .. }
        ));
    }
}
