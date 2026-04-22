use crate::{
    ast, ir,
    type_checker::{patterns::DesugaredPattern, TypeChecker},
    DiagnosticKind, Location, SymbolData, SymbolKind, TypeStore,
};

impl TypeChecker<'_> {
    pub fn visit_variable_declaration(
        &mut self,
        node: ast::VariableDeclaration,
    ) -> Vec<ir::VariableDeclaration> {
        let mutable = node.keyword == ast::DeclarationKeyword::Var;
        let Some(pattern) = node.pattern else {
            return vec![];
        };
        let Some(value) = node.value else {
            return vec![];
        };
        let pattern_loc = pattern.loc();
        let DesugaredPattern { test, bindings } = self.desugar_pattern(pattern, value, mutable);

        if test.is_some() {
            self.error(DiagnosticKind::IrrefutablePatternExpected, pattern_loc);
            return vec![];
        }

        // TODO create tmp assignment to avoid cloning potentially heavy/not idempotent expression.
        bindings
            .into_iter()
            .filter_map(|(identifier, value)| {
                self.visit_simple_declaration(
                    node.docs.clone(),
                    node.loc,
                    mutable,
                    identifier,
                    value,
                )
            })
            .collect::<Vec<_>>()
    }

    pub fn visit_simple_declaration(
        &mut self,
        docs: Option<ast::Docs>,
        loc: Location,
        mutable: bool,
        identifier: ast::Identifier,
        value: ast::Expression,
    ) -> Option<ir::VariableDeclaration> {
        self.check_identifier_sanity(&identifier);
        match self.ctx.find_in_current_scope(identifier.as_str()) {
            Some(symbol) => {
                let error = DiagnosticKind::DuplicateIdentifier {
                    name: identifier.as_str().to_string(),
                };
                self.error(error, identifier.loc);
                symbol.borrow().access.read(identifier.loc);
                None
            }
            None => {
                let value = self.visit_expression(value);
                let dependencies = value.as_ref().map_or(vec![], |value| {
                    value
                        .walk()
                        .filter_map(|e| e.as_identifier())
                        .map(|identifier| identifier.symbol.clone())
                        .collect::<Vec<_>>()
                });
                let symbol = self.ctx.register_symbol(SymbolData {
                    name: identifier.as_str().to_string(),
                    ty: value.as_ref().map_or(TypeStore::UNKNOWN, |v| v.ty()),
                    kind: SymbolKind::Value { mutable },
                    docs: docs.map(|d| d.text),
                    defined_at: identifier.loc,
                    dependencies: dependencies,
                    ..Default::default()
                });
                Some(ir::VariableDeclaration {
                    loc,
                    mutable,
                    symbol,
                    value: value?,
                })
            }
        }
    }

    fn check_identifier_sanity(&mut self, identifier: &ast::Identifier) {
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
            docs: None,
            loc: Location::dummy(),
            keyword: ast::DeclarationKeyword::Var,
            pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern(
                ast::Identifier {
                    text: "a".to_string(),
                    loc: Location::dummy(),
                },
            ))),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
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
            docs: None,
            loc: Location::dummy(),
            keyword: ast::DeclarationKeyword::Const,
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
            kind: SymbolKind::Value { mutable: true },
            ..Default::default()
        });
        let node = ast::VariableDeclaration {
            docs: None,
            loc: Location::dummy(),
            keyword: ast::DeclarationKeyword::Var,
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
            docs: None,
            loc: Location::dummy(),
            keyword: ast::DeclarationKeyword::Const,
            pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern(
                ast::Identifier {
                    loc: Location::dummy(),
                    text: "derived$".to_string(),
                },
            ))),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
        };
        let tc = visit_variable_declaration(&node);
        assert_eq!(tc.diagnostics.len(), 1);
        assert_eq!(
            tc.diagnostics[0].kind,
            DiagnosticKind::InvalidIdentifierDollar
        );
    }
}
