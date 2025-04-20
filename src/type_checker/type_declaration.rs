use crate::{
    ast::{AstNode, Node},
    parser::parser::ParseError,
    types::Type,
};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_type_declaration(&mut self, declaration: &AstNode) -> Type {
        let Node::TypeDeclaration {
            ref name,
            ref type_params,
            ref def,
        } = declaration.node
        else {
            panic!("Expected a type declaration");
        };

        self.type_registry.current_type_params = type_params.clone();
        let typ = match def {
            Some(def) => self.visit(&def),
            None => Type::Unknown,
        };
        self.type_registry.current_type_params = None;

        match self.type_registry.lookup(&name) {
            Some(_) => self.errors.push(ParseError {
                message: format!("Type {} already defined", name),
                span: declaration.span,
            }),
            None => self.type_registry.define(&name, typ),
        }

        Type::Void
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Node, Spanned};
    use crate::types::Type;
    use pest::Span;

    fn dummy_span() -> Span<'static> {
        Span::new("dummy", 0, 5).unwrap()
    }

    fn spanned(node: Node) -> AstNode {
        Spanned {
            node,
            span: dummy_span(),
        }
    }

    #[test]
    fn test_valid_type_declaration() {
        let mut checker = TypeChecker::new();

        let type_declaration = spanned(Node::TypeDeclaration {
            name: "MyType".to_string(),
            type_params: None,
            def: Some(Box::new(spanned(Node::NamedType("number".to_string())))),
        });

        let result = checker.visit_type_declaration(&type_declaration);

        assert!(checker.errors.is_empty());
        assert_eq!(result, Type::Void);
        assert!(
            matches!(checker.type_registry.lookup("MyType"), Some(_)),
            "Expected MyType to be defined in the type registry"
        );
    }

    #[test]
    fn test_duplicate_type_declaration() {
        let mut checker = TypeChecker::new();

        let type_declaration = spanned(Node::TypeDeclaration {
            name: "MyType".to_string(),
            type_params: None,
            def: Some(Box::new(spanned(Node::NamedType("number".to_string())))),
        });

        checker.visit_type_declaration(&type_declaration);
        checker.visit_type_declaration(&type_declaration);

        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0]
            .message
            .contains("Type MyType already defined"));
    }

    #[test]
    fn test_type_declaration_with_no_definition() {
        let mut checker = TypeChecker::new();

        let type_declaration = spanned(Node::TypeDeclaration {
            name: "MyType".to_string(),
            type_params: None,
            def: None,
        });

        let result = checker.visit_type_declaration(&type_declaration);

        assert!(checker.errors.is_empty());
        assert_eq!(result, Type::Void);
        assert!(matches!(
            checker.type_registry.lookup("MyType"),
            Some(Type::Unknown)
        ));
    }
}
