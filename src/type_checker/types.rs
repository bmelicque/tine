use crate::{
    ast::{AstNode, Node},
    parser::parser::ParseError,
    types::Type,
};

use super::TypeChecker;

impl TypeChecker {
    pub(super) fn visit_named_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::NamedType(name) = node else {
            panic!("Expected NamedType node")
        };

        match name.as_str() {
            "string" => Type::String,
            "number" => Type::Number,
            "boolean" => Type::Boolean,
            "void" => Type::Void,
            _ => match self.type_registry.lookup(&name) {
                Some(_) => Type::Named(name.clone()),
                None => {
                    self.errors.push(ParseError {
                        message: format!("Type '{}' not found", name),
                        span: ast_node.span.clone(),
                    });
                    Type::Unknown
                }
            },
        }
    }

    pub(super) fn visit_generic_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::GenericType { name, args } = node else {
            panic!("Expected GenericType node")
        };

        let base = self.visit_named_type(name);
        match base {
            Type::Generic { .. } | Type::Unknown => {}
            _ => {
                self.errors.push(ParseError {
                    message: format!("Expected a generic type, found {:?}", base),
                    span: ast_node.span.clone(),
                });
            }
        };

        let mut generic_args = Vec::new();
        for arg in args {
            let arg_type = self.visit(arg);
            generic_args.push(arg_type);
        }

        let name = if let Node::NamedType(name) = &name.node {
            name.clone()
        } else {
            panic!("Expected NamedType node for generic type name")
        };
        Type::Generic {
            name,
            args: generic_args,
        }
    }

    pub(super) fn visit_binary_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::BinaryType {
            left,
            operator,
            right,
        } = node
        else {
            panic!("Expected BinaryType node")
        };

        let left_type = match left {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };
        let right_type = match right {
            Some(spanned) => self.visit(spanned),
            None => Type::Dynamic,
        };

        match operator.as_str() {
            "#" => Type::Map {
                key: Box::new(left_type),
                value: Box::new(right_type),
            },
            "!" => Type::Result {
                error: Some(Box::new(left_type)),
                ok: Box::new(right_type),
            },
            _ => unreachable!("Unexpected operator in binary type: {}", operator),
        }
    }

    pub(super) fn visit_unary_type(&mut self, ast_node: &AstNode) -> Type {
        let node = &ast_node.node;
        let Node::UnaryType { operator, inner } = node else {
            panic!("Expected UnaryType node")
        };

        let operand_type = match inner {
            Some(spanned) => self.visit(&spanned),
            None => Type::Dynamic,
        };

        match operator.as_str() {
            "?" => Type::Option(Box::new(operand_type)),
            "&" => Type::Reference(Box::new(operand_type)),
            "[]" => Type::Array(Box::new(operand_type)),
            _ => unreachable!("Unexpected operator in unary type: {}", operator),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Node, Spanned};
    use crate::types::Type;
    use pest::Span;

    fn dummy_span() -> Span<'static> {
        let input = "dummy";
        Span::new(input, 0, input.len()).unwrap()
    }

    fn spanned(node: Node) -> Spanned<Node> {
        Spanned {
            node,
            span: dummy_span(),
        }
    }

    #[test]
    fn test_visit_named_type() {
        let mut checker = TypeChecker::new();

        // Test for built-in types
        let string_node = spanned(Node::NamedType("string".to_string()));
        let number_node = spanned(Node::NamedType("number".to_string()));
        let boolean_node = spanned(Node::NamedType("boolean".to_string()));
        let void_node = spanned(Node::NamedType("void".to_string()));

        assert!(matches!(
            checker.visit_named_type(&string_node),
            Type::String
        ));
        assert!(matches!(
            checker.visit_named_type(&number_node),
            Type::Number
        ));
        assert!(matches!(
            checker.visit_named_type(&boolean_node),
            Type::Boolean
        ));
        assert!(matches!(checker.visit_named_type(&void_node), Type::Void));

        // Test for user-defined types
        let custom_type_node = spanned(Node::NamedType("CustomType".to_string()));
        checker.type_registry.define("CustomType", Type::Number);
        let result = checker.visit_named_type(&custom_type_node);

        match result {
            Type::Named(name) => assert_eq!(name, "CustomType"),
            _ => panic!("Expected Named type, got {:?}", result),
        }
    }

    #[test]
    fn test_visit_unary_type_option() {
        let ast_node = spanned(Node::UnaryType {
            operator: "?".to_string(),
            inner: Some(Box::new(spanned(Node::NamedType("number".to_string())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.visit_unary_type(&ast_node);

        match result {
            Type::Option(inner) => {
                assert!(
                    matches!(*inner, Type::Number),
                    "expected number type, got {:?}",
                    inner
                );
            }
            _ => panic!("Expected Option type"),
        }
    }

    #[test]
    fn test_visit_unary_type_array() {
        let ast_node = spanned(Node::UnaryType {
            operator: "[]".to_string(),
            inner: Some(Box::new(spanned(Node::NamedType("string".to_string())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.visit_unary_type(&ast_node);

        match result {
            Type::Array(inner) => {
                assert!(matches!(*inner, Type::String));
            }
            _ => panic!("Expected Array type"),
        }
    }

    #[test]
    fn test_visit_binary_type_map() {
        let ast_node = spanned(Node::BinaryType {
            left: Some(Box::new(spanned(Node::NamedType("string".to_string())))),
            operator: "#".to_string(),
            right: Some(Box::new(spanned(Node::NamedType("number".to_string())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.visit_binary_type(&ast_node);

        match result {
            Type::Map { key, value } => {
                assert!(matches!(*key, Type::String));
                assert!(matches!(*value, Type::Number));
            }
            _ => panic!("Expected Map type"),
        }
    }

    #[test]
    fn test_visit_binary_type_result() {
        let ast_node = spanned(Node::BinaryType {
            left: Some(Box::new(spanned(Node::NamedType("string".to_string())))),
            operator: "!".to_string(),
            right: Some(Box::new(spanned(Node::NamedType("number".to_string())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.visit_binary_type(&ast_node);

        match result {
            Type::Result { error, ok } => {
                assert!(matches!(error.unwrap().as_ref(), Type::String));
                assert!(matches!(ok.as_ref(), Type::Number));
            }
            _ => panic!("Expected Result type"),
        }
    }

    #[test]
    fn test_visit_generic_type() {
        let ast_node = spanned(Node::GenericType {
            name: Box::new(spanned(Node::NamedType("List".to_string()))),
            args: vec![
                Box::new(spanned(Node::NamedType("number".to_string()))),
                Box::new(spanned(Node::NamedType("string".to_string()))),
            ],
        });

        let mut checker = TypeChecker::new();
        checker.type_registry.define(
            "List",
            Type::Generic {
                name: "List".to_string(),
                args: vec![Type::Number, Type::String],
            },
        );
        let result = checker.visit_generic_type(&ast_node);

        match result {
            Type::Generic { name, args } => {
                assert_eq!(name, "List");
                assert_eq!(args.len(), 2);
                assert!(matches!(args[0], Type::Number));
                assert!(matches!(args[1], Type::String));
            }
            _ => panic!("Expected Generic type"),
        }
    }
}
