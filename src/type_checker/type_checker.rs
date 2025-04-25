use super::scopes::{SymbolTable, TypeRegistry, VariableInfo};

use crate::ast::{AstNode, Node, Spanned};
use crate::parser::parser::ParseError;
use crate::types::Type;

pub struct TypeChecker {
    pub errors: Vec<ParseError>,
    pub symbols: SymbolTable,
    pub type_registry: TypeRegistry,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut symbols = SymbolTable::default();
        symbols.enter_scope();
        Self {
            errors: Vec::new(),
            symbols,
            type_registry: TypeRegistry::new(),
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

    pub(super) fn visit(&mut self, node: &AstNode) -> Type {
        match &node.node {
            Node::Program(statements) => {
                for stmt in statements {
                    self.visit(&stmt);
                }
                Type::Void
            }
            Node::Block(statements) => {
                for stmt in statements {
                    self.visit(&stmt);
                }
                // TODO: block expressions?
                Type::Void
            }
            Node::VariableDeclaration {
                name,
                op,
                initializer,
            } => {
                let inferred_type = if let Some(expr) = initializer {
                    self.visit(&expr)
                } else {
                    Type::Unknown
                };

                let mutable = match op.as_str() {
                    ":=" => true,
                    "::" => false,
                    _ => panic!("Unexpected declaration operator '{}'", op),
                };

                if let Some(n) = name {
                    self.symbols.define(&n, inferred_type.clone(), mutable);
                }
                inferred_type
            }
            Node::Assignment { name, value } => {
                let value_type = match value {
                    Some(v) => self.visit(v),
                    None => {
                        self.errors.push(ParseError {
                            message: "Expression expected".to_string(),
                            span: node.span,
                        });
                        Type::Unknown
                    }
                };

                let Some(name) = name else {
                    self.errors.push(ParseError {
                        message: "Missing assignee".to_string(),
                        span: node.span,
                    });
                    return Type::Unknown;
                };

                match self.symbols.lookup(name) {
                    Some(&VariableInfo { ref ty, mutable }) => {
                        if !mutable {
                            self.errors.push(ParseError {
                                message: "Cannot assign to immutable variable".to_string(),
                                span: node.span,
                            });
                        }
                        if ty != &value_type {
                            self.errors.push(ParseError {
                                message: format!(
                                    "Type mismatch in assignment to '{}': expected {:?}, found {:?}",
                                    name, ty, value_type
                                ),
                                span: node.span,
                            });
                            Type::Unknown
                        } else {
                            Type::Void
                        }
                    }
                    None => {
                        self.errors.push(ParseError {
                            message: format!("Assignment to undeclared variable '{}'", name),
                            span: node.span,
                        });
                        Type::Unknown
                    }
                }
            }
            Node::TypeDeclaration { .. } => self.visit_type_declaration(node),
            Node::ReturnStatement(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.visit(&expr)
                } else {
                    Type::Void
                }
            }
            Node::ExpressionStatement(expr) => self.visit(&expr),
            Node::FunctionExpression {
                parameters: _,
                return_type: _,
                body: _,
            } => self.visit_function_expression(&node.node),
            Node::BinaryExpression {
                left,
                operator,
                right,
            } => {
                let left_type = match left {
                    Some(expr) => self.visit(&expr),
                    None => Type::Unknown,
                };
                let right_type = match right {
                    Some(expr) => self.visit(&expr),
                    None => Type::Unknown,
                };

                if left_type != right_type
                    && left_type != Type::Unknown
                    && right_type != Type::Unknown
                {
                    self.errors.push(ParseError {
                        message: format!(
                            "Binary type mismatch: {:?} vs {:?}",
                            left_type, right_type
                        ),
                        span: node.span,
                    });
                    return Type::Unknown;
                }

                let valid = match operator.as_str() {
                    "+" | "-" | "*" | "**" | "/" | "%" | "<" | "<=" | ">" | ">=" => {
                        matches!(left_type, Type::Unknown | Type::Number)
                    }
                    "==" | "!=" => true,
                    "&&" | "||" => matches!(left_type, Type::Unknown | Type::Boolean),
                    _ => false,
                };
                if !valid {
                    self.errors.push(ParseError {
                        message: format!(
                            "Operator '{}' cannot be applied to type {:?}",
                            operator, left_type
                        ),
                        span: node.span,
                    });
                }

                match operator.as_str() {
                    "==" | "!=" | "<" | "<=" | ">" | ">=" | "&&" | "||" => Type::Boolean,
                    "+" | "-" | "*" | "**" | "/" | "%" => Type::Number,
                    _ => left_type,
                }
            }
            Node::Identifier(name) => match self.symbols.lookup(&name) {
                Some(VariableInfo { ty, mutable: _ }) => ty.clone(),
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

            Node::MapLiteral { .. } => self.visit_map_literal(node),
            Node::OptionLiteral { .. } => self.visit_option_literal(node),
            Node::ArrayLiteral { .. } => self.visit_array_literal(node),
            Node::StructLiteral { .. } => self.visit_struct_literal(node),

            Node::GenericType { .. } => self.visit_generic_type(node),
            Node::OptionType(_) => self.visit_option_type(node),
            Node::ReferenceType(_) => self.visit_reference_type(node),
            Node::ArrayType(_) => self.visit_array_type(node),
            Node::NamedType(_) => self.visit_named_type(node),
            Node::MapType { .. } => self.visit_map_type(node),
            Node::ResultType { .. } => self.visit_result_type(node),
            Node::FunctionType { .. } => self.visit_function_type(node),
            Node::TupleType(_) => self.visit_tuple_type(node),
            Node::Struct(_) => self.visit_struct_type(node),
            Node::SumDef(_) => self.visit_sum_def(node),
            Node::TraitDef { .. } => self.visit_trait_def(node),
        }
    }

    pub(super) fn resolve_type(&mut self, ast_node: &AstNode) -> Type {
        let Spanned { node, span } = &ast_node;
        match node {
            Node::Identifier(id) => match id.as_str() {
                "string" => Type::String,
                "number" => Type::Number,
                "boolean" => Type::Boolean,
                "void" => Type::Void,
                id => match self.type_registry.lookup(id) {
                    Some(ty) => ty.clone(),
                    None => {
                        self.errors.push(ParseError {
                            message: format!("Unknown type: {}", id),
                            span: *span,
                        });
                        Type::Unknown
                    }
                },
            },
            _ => panic!("Not implemented yet!"),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::{Node, Spanned};
    use crate::type_checker::TypeChecker;
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
    fn test_variable_declaration_and_lookup() {
        let ast = spanned(Node::Program(vec![
            spanned(Node::VariableDeclaration {
                name: Some("x".to_string()),
                op: ":=".to_string(),
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
    fn test_valid_assignment() {
        let mut checker = TypeChecker::new();
        checker.symbols.define("x", Type::Number, true);

        let node = spanned(Node::Assignment {
            name: Some("x".to_string()),
            value: Some(Box::new(spanned(Node::NumberLiteral(42.0)))),
        });

        let result = checker.check(&node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_assignment_to_constant() {
        let mut checker = TypeChecker::new();
        checker.symbols.define("x", Type::Number, false);

        let node = spanned(Node::Assignment {
            name: Some("x".to_string()),
            value: Some(Box::new(spanned(Node::NumberLiteral(42.0)))),
        });

        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors.len() == 1);
        assert!(checker.errors[0]
            .message
            .contains("Cannot assign to immutable variable"));
    }

    #[test]
    fn test_assignment_type_mismatch() {
        let mut checker = TypeChecker::new();
        checker.symbols.define("x", Type::String, true);

        let node = spanned(Node::Assignment {
            name: Some("x".to_string()),
            value: Some(Box::new(spanned(Node::BooleanLiteral(true)))),
        });

        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors[0]
            .message
            .contains("Type mismatch in assignment"));
    }

    #[test]
    fn test_assignment_to_undeclared_variable() {
        let mut checker = TypeChecker::new();

        let node = spanned(Node::Assignment {
            name: Some("y".to_string()),
            value: Some(Box::new(spanned(Node::NumberLiteral(99.0)))),
        });

        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors[0]
            .message
            .contains("Assignment to undeclared variable"));
    }

    #[test]
    fn test_assignment_missing_name() {
        let mut checker = TypeChecker::new();

        let node = spanned(Node::Assignment {
            name: None,
            value: Some(Box::new(spanned(Node::StringLiteral("oops".into())))),
        });

        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors[0].message.contains("Missing assignee"));
    }

    #[test]
    fn test_assignment_missing_value() {
        let mut checker = TypeChecker::new();
        checker.symbols.define("z", Type::Boolean, true);

        let node = spanned(Node::Assignment {
            name: Some("z".to_string()),
            value: None,
        });

        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors[0].message.contains("Expression expected"));
    }

    #[test]
    fn test_number_addition() {
        let node = spanned(Node::BinaryExpression {
            left: Some(Box::new(spanned(Node::NumberLiteral(1.0)))),
            operator: "+".to_string(),
            right: Some(Box::new(spanned(Node::NumberLiteral(2.0)))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.check(&node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_boolean_addition_error() {
        let node = spanned(Node::BinaryExpression {
            left: Some(Box::new(spanned(Node::BooleanLiteral(true)))),
            operator: "+".to_string(),
            right: Some(Box::new(spanned(Node::BooleanLiteral(false)))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors[0]
            .message
            .contains("Operator '+' cannot be applied"));
    }

    #[test]
    fn test_equality_check_valid() {
        let node = spanned(Node::BinaryExpression {
            left: Some(Box::new(spanned(Node::StringLiteral("a".into())))),
            operator: "==".to_string(),
            right: Some(Box::new(spanned(Node::StringLiteral("b".into())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.check(&node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_logical_and_valid() {
        let node = spanned(Node::BinaryExpression {
            left: Some(Box::new(spanned(Node::BooleanLiteral(true)))),
            operator: "&&".to_string(),
            right: Some(Box::new(spanned(Node::BooleanLiteral(false)))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.check(&node);
        assert!(result.is_ok());
    }

    #[test]
    fn test_logical_and_invalid_type() {
        let node = spanned(Node::BinaryExpression {
            left: Some(Box::new(spanned(Node::NumberLiteral(1.0)))),
            operator: "&&".to_string(),
            right: Some(Box::new(spanned(Node::NumberLiteral(0.0)))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors[0]
            .message
            .contains("Operator '&&' cannot be applied"));
    }

    #[test]
    fn test_type_mismatch_error() {
        let node = spanned(Node::BinaryExpression {
            left: Some(Box::new(spanned(Node::NumberLiteral(1.0)))),
            operator: "+".to_string(),
            right: Some(Box::new(spanned(Node::StringLiteral("oops".into())))),
        });

        let mut checker = TypeChecker::new();
        let result = checker.check(&node);
        assert!(result.is_err());
        assert!(checker.errors[0].message.contains("Binary type mismatch"));
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
