use crate::{
    ast,
    type_checker::{patterns::TokenList, TypeChecker},
    types::TypeId,
    DiagnosticKind, SymbolData, SymbolKind, TypeStore,
};

impl TypeChecker<'_> {
    pub fn visit_variable_declaration(&mut self, node: &ast::VariableDeclaration) -> TypeId {
        let (inferred_type, dependencies) = match &node.value {
            Some(value) => self.with_dependencies(|s| s.visit_expression(value)),
            None => (TypeStore::UNKNOWN, vec![]),
        };
        let Some(pattern) = &node.pattern else {
            return TypeStore::UNIT;
        };

        let mutable = node.keyword == ast::DeclarationKeyword::Var;
        if pattern.is_refutable() {
            self.error(DiagnosticKind::IrrefutablePatternExpected, pattern.loc());
        }
        let mut variables = TokenList::new();
        let docs = node.docs.clone().map(|d| d.text);
        self.match_pattern(pattern, inferred_type, &mut variables);
        for (id, ty) in variables.0 {
            self.check_identifier_sanity(&id);
            match self.ctx.find_in_current_scope(id.as_str()) {
                Some(symbol) => {
                    let error = DiagnosticKind::DuplicateIdentifier {
                        name: id.as_str().to_string(),
                    };
                    self.error(error, id.loc);
                    symbol
                }
                None => self.ctx.register_symbol(SymbolData {
                    name: id.as_str().to_string(),
                    ty,
                    kind: SymbolKind::Value { mutable },
                    docs: docs.clone(),
                    defined_at: pattern.loc(),
                    dependencies: dependencies.clone(),
                    ..Default::default()
                }),
            };
        }
        TypeStore::UNIT
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
        tc.visit_variable_declaration(node);
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
        tc.visit_variable_declaration(&node);
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
