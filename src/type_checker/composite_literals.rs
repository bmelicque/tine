use core::panic;
use std::collections::HashMap;

use crate::{
    ast::{AstNode, Node},
    parser::parser::ParseError,
    types::Type,
};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_map_literal(&mut self, map: &AstNode) -> Type {
        let Node::MapLiteral {
            ref entries,
            ref ty,
        } = map.node
        else {
            panic!("Expected a map literal");
        };

        let ty = self.visit(&ty);
        let Type::Map { mut key, mut value } = ty else {
            panic!("Expected a map type");
        };

        for entry in entries {
            let entry_key = self.visit(&entry.node.key);
            let entry_value = self.visit(&entry.node.value);

            match check_dynamic_type(&entry_key, &key) {
                Ok(ty) => key = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: entry.span,
                    });
                }
            }

            match check_dynamic_type(&entry_value, &value) {
                Ok(ty) => value = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: entry.span,
                    });
                }
            }
        }

        Type::Map { key, value }
    }

    pub fn visit_option_literal(&mut self, option: &AstNode) -> Type {
        let Node::OptionLiteral { ref ty, ref value } = option.node else {
            panic!("Expected an option type");
        };

        let ty = self.visit(ty);
        let Type::Option(mut inner) = ty else {
            panic!("Expected an option type");
        };

        if let Some(value) = value {
            let value_ty = self.visit(value);
            match check_dynamic_type(&value_ty, &inner) {
                Ok(ty) => inner = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: value.span,
                    });
                }
            }
        }

        if matches!(inner.as_ref(), Type::Dynamic) {
            self.errors.push(ParseError {
                message: "Cannot infer type".to_string(),
                span: option.span,
            });
            inner = Box::new(Type::Unknown);
        }

        Type::Option(inner)
    }

    pub fn visit_array_literal(&mut self, option: &AstNode) -> Type {
        let Node::ArrayLiteral {
            ref ty,
            ref elements,
        } = option.node
        else {
            panic!("Expected an option type");
        };

        let ty = self.visit(ty);
        let Type::Option(mut inner) = ty else {
            panic!("Expected an option type");
        };

        for value in elements {
            let value_ty = self.visit(value);
            match check_dynamic_type(&value_ty, &inner) {
                Ok(ty) => inner = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: value.span,
                    });
                }
            }
        }

        if matches!(inner.as_ref(), Type::Dynamic) {
            self.errors.push(ParseError {
                message: "Cannot infer type".to_string(),
                span: option.span,
            });
            inner = Box::new(Type::Unknown);
        }

        Type::Array(inner)
    }

    pub fn visit_struct_literal(&mut self, struct_literal: &AstNode) -> Type {
        let Node::StructLiteral {
            ref struct_type,
            ref fields,
        } = struct_literal.node
        else {
            panic!("Expected a struct literal");
        };

        let ty = self.visit(struct_type);
        let Type::Named(ref name) = ty else {
            panic!("Expected a named type");
        };

        let field_types = match self.type_registry.lookup(&name) {
            Some(Type::Struct { fields }) => fields,
            Some(ty) => {
                self.errors.push(ParseError {
                    message: format!("Expected a structured type, found {:?}", ty),
                    span: struct_literal.span,
                });
                return Type::Unknown;
            }
            None => panic!("Named type should refer to something"),
        };
        let mut field_map = HashMap::new();
        for field in field_types {
            let name = field.name.clone();
            let ty = field.def;
            field_map.insert(name, ty);
        }

        for field in fields {
            let name = &field.node.name;
            let value = self.visit(&field.node.value);
            let expected = field_map.get(name).cloned();
            let Some(expected) = expected else {
                self.errors.push(ParseError {
                    message: format!("Field {} not found in struct", name),
                    span: field.span,
                });
                continue;
            };
            match check_dynamic_type(&value, &expected) {
                Ok(ty) => {
                    if let Some(field) = field_map.get_mut(name) {
                        *field = *ty;
                    }
                }
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: field.span,
                    });
                }
            }
        }

        // Look for fields that are not in the struct definition
        for (name, _) in field_map {
            if !fields.iter().any(|f| f.node.name == name) {
                self.errors.push(ParseError {
                    message: "No such field found".to_string(),
                    span: struct_literal.span,
                });
            }
        }

        ty
    }
}

fn check_dynamic_type(ty: &Type, expected: &Type) -> Result<Box<Type>, String> {
    if matches!(ty, Type::Dynamic) {
        return Ok(Box::new(expected.clone()));
    }

    if ty.is_assignable_to(expected) {
        Ok(Box::new(expected.clone()))
    } else {
        Err(format!(
            "Key type mismatch: expected {}, found {}",
            expected, ty
        ))
    }
}
