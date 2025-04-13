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
            Node::VariableDeclaration {
                name,
                type_annotation,
                initializer,
            } => {
                let inferred_type = if let Some(expr) = initializer {
                    self.visit(expr)
                } else {
                    Type::Unknown
                };

                let declared_type = type_annotation
                    .as_ref()
                    .and_then(|ann| self.resolve_type(ann))
                    .unwrap_or(Type::Unknown);

                if declared_type != Type::Unknown
                    && inferred_type != Type::Unknown
                    && declared_type != inferred_type
                {
                    self.errors.push(TranspilerError {
                        message: format!(
                            "Type mismatch for '{}': expected {:?}, found {:?}",
                            name, declared_type, inferred_type
                        ),
                    });
                }

                self.symbols.define(name, declared_type.clone());
                declared_type
            }
            Node::FunctionDeclaration {
                name,
                params,
                return_type,
                body,
            } => {
                let mut param_types = Vec::new();

                for (param_name, param_type) in params {
                    let typ = self.resolve_type(param_type).unwrap_or(Type::Unknown);
                    self.symbols.define(param_name, typ.clone());
                    param_types.push(typ);
                }

                for stmt in body {
                    self.visit(stmt);
                }

                let return_typ = return_type
                    .as_ref()
                    .and_then(|r| self.resolve_type(r))
                    .unwrap_or(Type::Void);

                let func_type = Type::Function {
                    params: param_types,
                    return_type: Box::new(return_typ.clone()),
                };

                self.symbols.define(name, func_type);
                return_typ
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

                let left_type = self.visit(left);
                let right_type = self.visit(right);

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
            Node::FunctionCall { name, args } => {
                let Some(func_type) = self.symbols.lookup(name).cloned() else {
                    self.errors.push(TranspilerError {
                        message: format!("Undefined function: {}", name),
                    });
                    return Type::Unknown;
                };

                let Type::Function {
                    params,
                    return_type,
                } = &func_type
                else {
                    self.errors.push(TranspilerError {
                        message: format!("'{}' is not a function", name),
                    });
                    return Type::Unknown;
                };

                if params.len() != args.len() {
                    self.errors.push(TranspilerError {
                        message: format!(
                            "Function '{}' expects {} arguments, but {} were provided",
                            name,
                            params.len(),
                            args.len()
                        ),
                    });
                    return Type::Unknown;
                }

                for (arg, param_type) in args.iter().zip(params) {
                    let arg_type = self.visit(arg);
                    if &arg_type != param_type {
                        self.errors.push(TranspilerError {
                            message: format!(
                                "Type mismatch in argument for '{}': expected {:?}, found {:?}",
                                name, param_type, arg_type
                            ),
                        });
                    }
                }

                *return_type.clone()
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
