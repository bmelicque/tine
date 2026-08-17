use tine_ast as ast;
use tine_common::locations::Locatable;
use tine_ir as ir;

use crate::TypeChecker;

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

        let Some(pattern) = self.visit_pattern(pattern, &value, false) else {
            return vec![];
        };

        if !self.check_exhaustiveness(vec![&pattern], pattern.loc()) {
            return vec![];
        }
        vec![ir::VariableDeclaration {
            loc: node.loc,
            pattern,
            value,
        }]
    }
}

#[cfg(test)]
mod tests {
    use tine_common::{diagnostics::DiagnosticKind, locations::Location};
    use tine_types::store::TypeStore;

    use super::*;

    fn visit_variable_declaration(node: ast::VariableDeclaration) -> TypeChecker {
        let mut tc = TypeChecker::new();
        tc.visit_variable_declaration(node);
        tc
    }

    #[test]
    fn test_variable_declaration() {
        let node = ast::VariableDeclaration {
            mutable: true,
            pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern {
                loc: Location::dummy(),
                mutable: true,
                identifier: ast::Identifier::new("a".to_string(), Location::dummy()),
            })),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            ..Default::default()
        };
        let mut tc = visit_variable_declaration(node);
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
            pattern: Some(ast::Pattern::Identifier(
                ast::Identifier::new("a".to_string(), Location::dummy()).into(),
            )),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral::new(
                1,
                Location::dummy(),
            ))),
            ..Default::default()
        };
        let mut tc = visit_variable_declaration(node);
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
            pattern: Some(ast::Pattern::Identifier(
                ast::Identifier::new("computed$".to_string(), Location::dummy()).into(),
            )),
            value: Some(ast::Expression::IntLiteral(ast::IntLiteral {
                value: 1,
                loc: Location::dummy(),
            })),
            ..Default::default()
        };
        let tc = visit_variable_declaration(node);
        assert_eq!(tc.diagnostics.len(), 1);
        assert_eq!(
            tc.diagnostics[&0][0].kind,
            DiagnosticKind::InvalidIdentifierDollar
        );
    }
}
