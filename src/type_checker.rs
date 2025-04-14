use crate::ast::{AstNode, Node};
use crate::parser::parser::ParseError;
use crate::types::Type;
use std::collections::HashMap;

#[derive(Default)]
pub struct SymbolTable {
    symbols: HashMap<String, Type>,
}

impl SymbolTable {
    pub fn define(&mut self, name: &str, type_: Type) {
        self.symbols.insert(name.to_string(), type_);
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        self.symbols.get(name)
    }
}

pub struct TypeChecker {
    errors: Vec<ParseError>,
    symbols: SymbolTable,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            symbols: SymbolTable::default(),
        }
    }

    pub fn check(&mut self, node: &AstNode) -> Result<(), Vec<ParseError>> {
        self.visit(node);
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn visit(&mut self, node: &AstNode) -> Type {
        match &node.node {
            Node::Program(statements) => {
                for stmt in statements {
                    self.visit(&stmt);
                }
                Type::Void
            }
            Node::VariableDeclaration { name, initializer } => {
                let inferred_type = if let Some(expr) = initializer {
                    self.visit(&expr)
                } else {
                    Type::Unknown
                };

                if let Some(n) = name {
                    self.symbols.define(&n, inferred_type.clone());
                }
                inferred_type
            }
            Node::ReturnStatement(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.visit(&expr)
                } else {
                    Type::Void
                }
            }
            Node::ExpressionStatement(expr) => self.visit(&expr),
            Node::BinaryExpression {
                left,
                operator,
                right,
            } => {
                // FIXME: make sure that types are compatible with operator
                _ = operator;

                let left_type = match left {
                    Some(expr) => self.visit(&expr),
                    None => Type::Unknown,
                };
                let right_type = match right {
                    Some(expr) => self.visit(&expr),
                    None => Type::Unknown,
                };

                if left_type != right_type {
                    self.errors.push(ParseError {
                        message: format!(
                            "Binary type mismatch: {:?} vs {:?}",
                            left_type, right_type
                        ),
                        span: node.span,
                    });
                    Type::Unknown
                } else {
                    left_type
                }
            }
            Node::Identifier(name) => match self.symbols.lookup(&name) {
                Some(t) => t.clone(),
                None => {
                    self.errors.push(ParseError {
                        message: format!("Undefined variable: {}", name),
                        span: node.span,
                    });
                    Type::Unknown
                }
            },
            Node::StringLiteral(_) => Type::String,
            Node::NumberLiteral(_) => Type::Number,
            Node::BooleanLiteral(_) => Type::Boolean,
        }
    }

    // fn resolve_type(&self, type_str: &str) -> Option<Type> {
    //     match type_str {
    //         "string" => Some(Type::String),
    //         "number" => Some(Type::Number),
    //         "boolean" => Some(Type::Boolean),
    //         "void" => Some(Type::Void),
    //         _ => Some(Type::Unknown),
    //     }
    // }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Node, Spanned};
    use crate::type_checker::TypeChecker;
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
    fn test_variable_declaration_and_lookup() {
        let ast = spanned(Node::Program(vec![
            spanned(Node::VariableDeclaration {
                name: Some("x".to_string()),
                initializer: Some(Box::new(spanned(Node::NumberLiteral(1.0)))),
            }),
            spanned(Node::ExpressionStatement(Box::new(spanned(
                Node::Identifier("x".to_string()),
            )))),
        ]));

        let mut checker = TypeChecker::new();
        let result = checker.check(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_type_mismatch_error() {
        let ast = spanned(Node::Program(vec![
            spanned(Node::VariableDeclaration {
                name: Some("a".to_string()),
                initializer: Some(Box::new(spanned(Node::StringLiteral("hello".to_string())))),
            }),
            spanned(Node::BinaryExpression {
                left: Some(Box::new(spanned(Node::Identifier("a".to_string())))),
                operator: "+".to_string(),
                right: Some(Box::new(spanned(Node::NumberLiteral(42.0)))),
            }),
        ]));

        let mut checker = TypeChecker::new();
        let result = checker.check(&ast);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("Binary type mismatch")));
    }

    #[test]
    fn test_undefined_variable_error() {
        let ast = spanned(Node::Program(vec![spanned(Node::ExpressionStatement(
            Box::new(spanned(Node::Identifier("y".to_string()))),
        ))]));

        let mut checker = TypeChecker::new();
        let result = checker.check(&ast);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors
            .iter()
            .any(|e| e.message.contains("Undefined variable")));
    }

    #[test]
    fn test_return_void() {
        let ast = spanned(Node::Program(vec![spanned(Node::ReturnStatement(None))]));

        let mut checker = TypeChecker::new();
        let result = checker.check(&ast);
        assert!(result.is_ok());
    }

    #[test]
    fn test_return_with_value() {
        let ast = spanned(Node::Program(vec![spanned(Node::ReturnStatement(Some(
            Box::new(spanned(Node::BooleanLiteral(true))),
        )))]));

        let mut checker = TypeChecker::new();
        let result = checker.check(&ast);
        assert!(result.is_ok());
    }
}
