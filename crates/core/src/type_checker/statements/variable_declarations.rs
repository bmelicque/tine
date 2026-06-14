use crate::{
    ast, ir,
    type_checker::{
        patterns::{Binding, DesugaredPattern},
        utils::make_tmp_identifier,
        TypeChecker,
    },
    DiagnosticKind, Location, SymbolData, SymbolKind, TypeStore,
};

impl TypeChecker<'_> {
    pub fn visit_variable_declaration(
        &mut self,
        node: ast::VariableDeclaration,
    ) -> Vec<ir::VariableDeclaration> {
        let Some(pattern) = &node.pattern else {
            return vec![];
        };
        let Some(value) = &node.value else {
            return vec![];
        };
        let loc = node.loc;
        let pattern_loc = pattern.loc();
        let docs = node.docs.clone();

        let (prelim, desugared) = match pattern {
            ast::Pattern::Identifier(_) | ast::Pattern::MutIdentifier(_) => {
                (node, DesugaredPattern::default())
            }
            _ => {
                let tmp_id = make_tmp_identifier(value.loc());
                let prelim = ast::VariableDeclaration {
                    loc: value.loc(),
                    pattern: Some(ast::Pattern::Identifier(tmp_id.clone().into())),
                    annotation: node.annotation.clone(),
                    value: Some(value.to_owned()),
                    ..Default::default()
                };
                let pattern = node.pattern.unwrap();
                (prelim, self.desugar_pattern(pattern, tmp_id.into()))
            }
        };

        let DesugaredPattern { test, bindings } = desugared;

        if test.is_some() {
            self.error(DiagnosticKind::IrrefutablePatternExpected, pattern_loc);
            return vec![];
        }

        let prelim = self.visit_prelim_declaration(prelim);

        let decls = bindings
            .into_iter()
            .filter_map(|binding| self.visit_binding(docs.clone(), loc, binding));

        prelim.into_iter().chain(decls).collect::<Vec<_>>()
    }

    fn visit_prelim_declaration(
        &mut self,
        decl: ast::VariableDeclaration,
    ) -> Option<ir::VariableDeclaration> {
        let pattern = decl.pattern?;
        let annotation = decl.annotation.map(|a| self.visit_type(a));
        let value = decl.value.and_then(|v| self.visit_expression(v))?;
        if let Some(annotation) = annotation {
            let ok = self.can_be_assigned_to(annotation, value.ty());
            if !ok {
                let left_name = self.session.display_type(annotation);
                let right_name = self.session.display_type(value.ty());
                let diag = DiagnosticKind::MismatchedTypes {
                    left_name,
                    right_name,
                };
                self.error(diag, decl.loc);
            }
        };
        let mutable = match &pattern {
            ast::Pattern::Identifier(_) => false,
            ast::Pattern::MutIdentifier(_) => true,
            _ => panic!(),
        };
        let identifier: ast::Identifier = match pattern {
            ast::Pattern::Identifier(id) => id.into(),
            ast::Pattern::MutIdentifier(id) => id.into(),
            _ => panic!(),
        };
        self.check_identifier_sanity(&identifier);

        match self.ctx.find_in_current_scope(identifier.as_str()) {
            Some(symbol) => {
                let error = DiagnosticKind::DuplicateIdentifier {
                    name: identifier.as_str().to_string(),
                };
                self.error(error, identifier.loc);
                symbol.borrow().access.read(identifier.loc);
                return None;
            }
            None => {
                let dependencies = value
                    .dependencies()
                    .map(|identifier| identifier.symbol.clone())
                    .collect::<Vec<_>>();
                let symbol = self.ctx.register_symbol(SymbolData {
                    name: identifier.as_str().to_string(),
                    ty: value.ty(),
                    kind: SymbolKind::Value { mutable },
                    defined_at: identifier.loc,
                    dependencies,
                    ..Default::default()
                });
                Some(ir::VariableDeclaration {
                    loc: decl.loc,
                    mutable: false,
                    symbol,
                    value,
                })
            }
        }
    }

    pub fn visit_binding(
        &mut self,
        docs: Option<ast::Docs>,
        loc: Location,
        binding: Binding,
    ) -> Option<ir::VariableDeclaration> {
        let Binding {
            mutable,
            id: identifier,
            value,
        } = binding;
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
                        .dependencies()
                        .map(|identifier| identifier.symbol.clone())
                        .collect::<Vec<_>>()
                });
                let symbol = self.ctx.register_symbol(SymbolData {
                    name: identifier.as_str().to_string(),
                    ty: value.as_ref().map_or(TypeStore::UNKNOWN, |v| v.ty()),
                    kind: SymbolKind::Value { mutable },
                    docs: docs.map(|d| d.text),
                    defined_at: identifier.loc,
                    dependencies,
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
