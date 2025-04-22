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

        let ty = if let Some(ty) = ty {
            self.visit(&ty)
        } else {
            Type::Unknown
        };

        let (mut key_type, mut value_type) = match ty.clone() {
            Type::Map { key, value } => (key, value),
            _ => {
                self.errors.push(ParseError {
                    message: "Expected a map type".to_string(),
                    span: map.span,
                });
                (Box::new(Type::Unknown), Box::new(Type::Unknown))
            }
        };

        for entry in entries {
            let key = self.visit(&entry.node.key);
            let value = self.visit(&entry.node.value);

            match check_dynamic_type(&key, &key_type) {
                Ok(ty) => key_type = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: entry.span,
                    });
                }
            }

            match check_dynamic_type(&value, &value_type) {
                Ok(ty) => value_type = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: entry.span,
                    });
                }
            }
        }

        match ty {
            Type::Map { .. } => Type::Map {
                key: key_type,
                value: value_type,
            },
            _ => Type::Unknown,
        }
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
