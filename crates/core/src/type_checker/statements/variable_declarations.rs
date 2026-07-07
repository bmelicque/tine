use crate::{
    ast, ir,
    type_checker::{patterns::lower_pattern, TypeChecker},
    DiagnosticKind,
};

impl TypeChecker {
    pub fn visit_variable_declaration(
        &mut self,
        node: ast::VariableDeclaration,
    ) -> Vec<ir::VariableDeclaration> {
        let Some(value) = node.value.and_then(|v| self.visit_expression(v)) else {
            return vec![];
        };
        let Some(pattern) = node.pattern else {
            return vec![];
        };
        let loc = node.loc;

        let (mut stmts, value) = self.make_temp_var_if_needed(&pattern, value);

        let Some(pattern) = self.visit_pattern(pattern, &value, true) else {
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

    /// When lowering the pattern, if complex (ie it binds several variables),
    /// then the original value needs to be placed in a temporary value, to
    /// avoid unwanted computation.
    ///
    /// # Example
    ///
    /// ```tine
    /// let User{ age, name } = fetchUser()
    ///
    /// // Bad
    /// let age = fetchUser().age
    /// let name = fetchUser().name
    ///
    /// // Good
    /// let tmp = fetchUser()
    /// let age = tmp.age
    /// let name = tmp.name
    /// ```
    fn make_temp_var_if_needed(
        &mut self,
        pattern: &ast::Pattern,
        value: ir::Expression,
    ) -> (Vec<ir::VariableDeclaration>, ir::Expression) {
        use ast::Pattern::*;
        if !matches!(pattern, Constructor(_) | Tuple(_)) {
            return (vec![], value);
        }

        let symbol = self.make_temp_variable(value.loc(), &value).1;
        let loc = value.loc();
        let ty = value.ty();
        let decl = ir::VariableDeclaration {
            loc,
            mutable: false,
            symbol,
            value,
        };
        let id = ir::Identifier {
            loc,
            symbol: symbol.into(),
            ty,
        };

        (vec![decl], id.into())
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
        type_checker::{type_store::TypeStore, TypeChecker},
        DiagnosticKind, Location,
    };

    fn visit_variable_declaration(node: &ast::VariableDeclaration) -> TypeChecker {
        let mut tc = TypeChecker::new();
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
        let mut tc = visit_variable_declaration(&node);
        let symbol = tc
            .current_scope()
            .lookup("a")
            .map(|id| tc.symbols.get_symbol(id));
        match symbol {
            Some(symbol) => {
                assert_eq!(symbol.ty(), TypeStore::INTEGER);
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
        let mut tc = visit_variable_declaration(&node);
        let symbol = tc
            .current_scope()
            .lookup("a")
            .map(|id| tc.symbols.get_symbol(id));
        match symbol {
            Some(symbol) => {
                assert_eq!(symbol.ty(), TypeStore::INTEGER);
            }
            None => {
                panic!("symbol not found")
            }
        }
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
            tc.diagnostics[&0][0].kind,
            DiagnosticKind::InvalidIdentifierDollar
        );
    }
}
