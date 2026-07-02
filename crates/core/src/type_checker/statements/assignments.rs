use crate::{
    ast::{self, Pattern},
    ir::{self, root_identifier},
    type_checker::{patterns::lower_pattern, type_store::TypeStore, TypeChecker},
    types::{self, Type, TypeId},
    DiagnosticKind,
};

impl TypeChecker {
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

                let id: ir::Expression = self.make_temp_variable(loc, &value).0.into();
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
        let ty = self.symbol_type_id(identifier.symbol);
        self.check_mutability(&identifier);

        self.check_assigned_type(ty, against, false, identifier.loc);
        Some(identifier.into())
    }

    fn visit_expr_assignee(
        &mut self,
        expr: ast::MemberExpression,
        against: TypeId,
    ) -> Option<ir::Expression> {
        let expression = self.visit_member_expression(expr)?.into();
        if let Some(root) = root_identifier(&expression) {
            // visit expression adds a read that need to be converted to write
            self.symbols
                .get_symbol_mut(root.symbol)
                .access()
                .read_to_write(root.loc);

            self.check_mutability(root);
        }
        self.check_assigned_type(against, expression.ty(), false, expression.loc());
        Some(expression)
    }

    fn check_mutability(&mut self, id: &ir::Identifier) {
        if !self.symbols.is_mutable(id.symbol) {
            let name = self.symbol_name(id.symbol).to_string();
            let error = DiagnosticKind::AssignmentToConstant { name };
            self.error(error, id.loc);
        }
    }

    fn visit_indirect_assignee(
        &mut self,
        node: ast::IndirectionAssignee,
        against: TypeId,
    ) -> Option<ir::Expression> {
        let name = node.identifier.as_str();
        let Some(symbol_id) = self.get_symbol_id(name) else {
            let error = DiagnosticKind::CannotFindName {
                name: name.to_string(),
            };
            self.error(error, node.identifier.loc);
            return None;
        };
        self.symbols
            .get_symbol_mut(symbol_id)
            .access()
            .write(node.identifier.loc);
        let symbol_ty = self.symbol_type_id(symbol_id);
        let ty = match self.resolve(symbol_ty).clone() {
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
                    type_name: self.types.display(symbol_ty),
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
                symbol: symbol_id,
                ty: symbol_ty,
            })),
            ty,
        }))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast,
        type_checker::{symbols::*, type_store::TypeStore, TypeChecker},
        DiagnosticKind, Location,
    };

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
        let mut checker = TypeChecker::new();
        checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "a".to_string(),
            ty: TypeStore::INTEGER,
            mutable: true,
            ..Default::default()
        });

        checker.visit_assignment(dummy_assignment());
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn visit_assignment_to_constant() {
        let mut checker = TypeChecker::new();
        checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "a".to_string(),
            ty: TypeStore::INTEGER,
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
        let mut checker = TypeChecker::new();
        checker.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name: "a".to_string(),
            ty: TypeStore::FLOAT,
            mutable: true,
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
        let mut checker = TypeChecker::new();
        checker.visit_assignment(dummy_assignment());
        assert_eq!(checker.diagnostics.len(), 1);
        assert!(matches!(
            &checker.diagnostics[0].kind,
            DiagnosticKind::CannotFindName { .. }
        ));
    }
}
