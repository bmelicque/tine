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

    pub fn visit_unary_literal(&mut self, unary: &AstNode) -> Type {
        let Node::UnaryLiteral {
            ref unary_type,
            ref body,
        } = unary.node
        else {
            panic!("Expected a unary literal");
        };

        let unary_type = self.visit(unary_type);
        let mut inner_type = match unary_type.clone() {
            Type::Array(inner) | Type::Option(inner) => inner,
            _ => {
                self.errors.push(ParseError {
                    message: "Expected an array or option type".to_string(),
                    span: unary.span,
                });
                Box::new(Type::Unknown)
            }
        };

        match unary_type {
            Type::Option(_) => {
                if body.len() > 1 {
                    self.errors.push(ParseError {
                        message: "Option literal must have at most one element".to_string(),
                        span: unary.span,
                    });
                }
            }
            _ => unreachable!(),
        }

        for expr in body {
            let ty = self.visit(expr);
            match check_dynamic_type(&ty, &inner_type) {
                Ok(ty) => inner_type = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: expr.span,
                    });
                }
            }
        }

        unary_type
    }

    // pub fn visit_struct_literal(&mut self, struct_literal: &AstNode) -> Type {
    //     let Node::StructLiteral {
    //         ref struct_type,
    //         ref fields,
    //     } = struct_literal.node
    //     else {
    //         panic!("Expected a struct literal");
    //     };

    //     let ty = self.visit(struct_type);

    //     let mut fields = match ty.clone() {
    //         Type::Struct(fields) => fields,
    //         _ => {
    //             self.errors.push(ParseError {
    //                 message: "Expected a struct type".to_string(),
    //                 span: struct_literal.span,
    //             });
    //             return Type::Unknown;
    //         }
    //     };

    //     for entry in body {
    //         let key = self.visit(&entry.node.key);
    //         let value = self.visit(&entry.node.value);

    //         if let Some(field) = fields.get_mut(&key.to_string()) {
    //             match check_dynamic_type(&value, field) {
    //                 Ok(ty) => *field = ty,
    //                 Err(message) => {
    //                     self.errors.push(ParseError {
    //                         message,
    //                         span: entry.span,
    //                     });
    //                 }
    //             }
    //         } else {
    //             self.errors.push(ParseError {
    //                 message: format!("Field {} not found in struct", key),
    //                 span: entry.span,
    //             });
    //         }
    //     }

    //     ty
    // }
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
