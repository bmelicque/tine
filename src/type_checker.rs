use crate::ast::Node;
use crate::transpiler::TranspilerError;
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
    errors: Vec<TranspilerError>,
    symbols: SymbolTable,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            symbols: SymbolTable::default(),
        }
    }

    pub fn check(&mut self, node: &Node) -> Result<(), Vec<TranspilerError>> {
        self.visit(node);
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    fn visit(&mut self, node: &Node) -> Type {
        match node {
            Node::Program(statements) => {
                for stmt in statements {
                    self.visit(stmt);
                }
                Type::Void
            }
            Node::VariableDeclaration { name, initializer } => {
                let inferred_type = if let Some(expr) = initializer {
                    self.visit(expr)
                } else {
                    Type::Unknown
                };

                if let Some(n) = name {
                    self.symbols.define(n, inferred_type.clone());
                }
                inferred_type
            }
            Node::ReturnStatement(expr_opt) => {
                if let Some(expr) = expr_opt {
                    self.visit(expr)
                } else {
                    Type::Void
                }
            }
            Node::ExpressionStatement(expr) => self.visit(expr),
            Node::BinaryExpression {
                left,
                operator,
                right,
            } => {
                // FIXME: make sure that types are compatible with operator
                _ = operator;

                let left_type = match left {
                    Some(expr) => self.visit(expr),
                    None => Type::Unknown,
                };
                let right_type = match right {
                    Some(expr) => self.visit(expr),
                    None => Type::Unknown,
                };

                if left_type != right_type {
                    self.errors.push(TranspilerError {
                        message: format!(
                            "Binary type mismatch: {:?} vs {:?}",
                            left_type, right_type
                        ),
                    });
                    Type::Unknown
                } else {
                    left_type
                }
            }
            Node::Identifier(name) => match self.symbols.lookup(name) {
                Some(t) => t.clone(),
                None => {
                    self.errors.push(TranspilerError {
                        message: format!("Undefined variable: {}", name),
                    });
                    Type::Unknown
                }
            },
            Node::StringLiteral(_) => Type::String,
            Node::NumberLiteral(_) => Type::Number,
            Node::BooleanLiteral(_) => Type::Boolean,
        }
    }

    fn resolve_type(&self, type_str: &str) -> Option<Type> {
        match type_str {
            "string" => Some(Type::String),
            "number" => Some(Type::Number),
            "boolean" => Some(Type::Boolean),
            "void" => Some(Type::Void),
            _ => Some(Type::Unknown),
        }
    }
}
