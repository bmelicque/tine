use crate::{
    ast, ir,
    type_checker::{patterns::lower_pattern, TypeChecker},
    DiagnosticKind,
};

impl TypeChecker<'_> {
    pub fn visit_variable_declaration(
        &mut self,
        node: ast::VariableDeclaration,
    ) -> Vec<ir::VariableDeclaration> {
        let value = node.value.and_then(|v| self.visit_expression(v));
        let Some(pattern) = node.pattern else {
            return vec![];
        };
        let loc = node.loc;

        let (mut stmts, value) = self.handle_prelim_stmt(&pattern, value);

        let pattern = value
            .as_ref()
            .and_then(|v| self.visit_pattern(pattern, v, true));
        let (Some(pattern), Some(value)) = (pattern, value) else {
            return vec![];
        };

        let lowered = lower_pattern(pattern, value);
        if lowered.test.is_some() {
            self.error(DiagnosticKind::IrrefutablePatternExpected, loc);
            return vec![];
        }
        stmts.extend(lowered.decls);
        stmts
    }

    fn handle_prelim_stmt(
        &mut self,
        pattern: &ast::Pattern,
        value: Option<ir::Expression>,
    ) -> (Vec<ir::VariableDeclaration>, Option<ir::Expression>) {
        use ast::Pattern::*;
        let Some(value) = value else {
            return (vec![], None);
        };
        if !matches!(pattern, Constructor(_) | Tuple(_)) {
            return (vec![], Some(value));
        }

        let id = self.make_temp_variable(value.loc(), &value);
        let decl = ir::VariableDeclaration {
            loc: value.loc(),
            mutable: false,
            symbol: id.symbol.clone(),
            value,
        };
        (vec![decl], Some(id.into()))
    }

    pub(crate) fn check_identifier_sanity(&mut self, identifier: &ast::Identifier) {
        if identifier.as_str().contains("$") {
            self.error(DiagnosticKind::InvalidIdentifierDollar, identifier.loc);
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

    fn visit_variable_declaration(node: &ast::VariableDeclaration) -> TypeChecker<'_> {
        let session = Session::new(Box::new(MockLoader));
        let mut tc = TypeChecker::new(Box::leak(Box::new(session)), 0);
        tc.visit_variable_declaration(node.clone());
        tc
    }

    #[test]
    fn test_variable_declaration() {
        let node = ast::VariableDeclaration {
            mutable: true,
            pattern: Some(ast::Pattern::MutIdentifier(ast::MutIdentifierPattern {
                loc: Location::dummy(),
                identifier: ast::IdentifierPattern::from(ast::Identifier {
                    text: "a".to_string(),
                    loc: Location::dummy(),
                }),
            })),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            ..Default::default()
        };
        let tc = visit_variable_declaration(&node);
        match tc.ctx.find_in_current_scope("a") {
            Some(symbol) => {
                assert_eq!(symbol.borrow().ty, TypeStore::INTEGER);
                assert_eq!(symbol.borrow().is_mutable(), true);
            }
            None => {
                panic!("symbol not found")
            }
        }
    }

    #[test]
    fn test_constant_declaration() {
        let node = ast::VariableDeclaration {
            pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern(
                ast::Identifier {
                    loc: Location::dummy(),
                    text: "a".to_string(),
                },
            ))),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            ..Default::default()
        };
        let tc = visit_variable_declaration(&node);
        match tc.ctx.find_in_current_scope("a") {
            Some(symbol) => {
                assert_eq!(symbol.borrow().ty, TypeStore::INTEGER);
                assert_eq!(symbol.borrow().is_mutable(), false);
            }
            None => {
                panic!("symbol not found")
            }
        }
    }

    #[test]
    fn test_duplicate_declaration() {
        let session = Session::new(Box::new(MockLoader));
        let mut tc = TypeChecker::new(&session, 0);
        tc.ctx.register_symbol(SymbolData {
            name: "a".to_string(),
            ty: TypeStore::INTEGER,
            kind: SymbolKind::constant(),
            ..Default::default()
        });
        let node = ast::VariableDeclaration {
            pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern(
                ast::Identifier {
                    loc: Location::dummy(),
                    text: "a".to_string(),
                },
            ))),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            ..Default::default()
        };
        tc.visit_variable_declaration(node);
        assert_eq!(tc.diagnostics.len(), 1);
        assert!(matches!(
            tc.diagnostics[0].kind,
            DiagnosticKind::DuplicateIdentifier { .. }
        ));
    }

    #[test]
    fn test_dollar_declaration() {
        let node = ast::VariableDeclaration {
            pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern(
                ast::Identifier {
                    loc: Location::dummy(),
                    text: "computed$".to_string(),
                },
            ))),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            ..Default::default()
        };
        let tc = visit_variable_declaration(&node);
        assert_eq!(tc.diagnostics.len(), 1);
        assert_eq!(
            tc.diagnostics[0].kind,
            DiagnosticKind::InvalidIdentifierDollar
        );
    }
}
