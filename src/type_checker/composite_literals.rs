use core::panic;
use std::collections::HashMap;

use crate::{
    ast::{AstNode, FieldAssignment, Node, Spanned},
    parser::parser::ParseError,
    types::Type,
};

use super::{scopes::TypeRegistry, TypeChecker};

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

    pub fn visit_array_literal(&mut self, array: &AstNode) -> Type {
        let Node::ArrayLiteral {
            ref ty,
            ref elements,
        } = array.node
        else {
            panic!("Expected an array literal");
        };

        let ty = self.visit(ty);
        let Type::Array(mut inner) = ty else {
            panic!("Expected an array type");
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
                span: array.span,
            });
            inner = Box::new(Type::Unknown);
        }

        Type::Array(inner)
    }

    pub fn visit_anonymous_array_literal(&mut self, spanned: &AstNode) -> Type {
        let Node::AnonymousArrayLiteral(ref elements) = spanned.node else {
            panic!("Expected an option type");
        };

        let mut ty = Type::Dynamic;
        for value in elements {
            let value_ty = self.visit(value);
            if ty == Type::Dynamic {
                ty = value_ty;
                continue;
            }
            if !value_ty.is_assignable_to(&ty) {
                self.errors.push(ParseError {
                    message: format!("Key type mismatch: expected {}, found {}", ty, value_ty),
                    span: value.span,
                })
            }
        }

        Type::Array(Box::new(ty))
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
        let Type::Named { ref name, ref args } = ty else {
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

        // setup generics registry
        let names = self.type_registry.get_type_params(name);
        let mut generics_registry = TypeRegistry::create(&names, &args);

        // set up map of expected field types
        let mut field_map = HashMap::new();
        for field in field_types {
            let name = field.name.clone();
            let ty = field.def;
            field_map.insert(name, ty);
        }

        for field in fields {
            self.visit_field_assignment(field, &field_map, &mut generics_registry);
        }

        self.report_unknown_fields(fields, &field_map);

        // TODO: adjust type using inferred generics
        ty
    }

    fn visit_field_assignment(
        &mut self,
        field: &Spanned<FieldAssignment>,
        expected_map: &HashMap<String, Type>,
        generics_registry: &mut TypeRegistry,
    ) {
        let name = &field.node.name;
        let value = self.visit(&field.node.value);
        let expected = expected_map.get(name).cloned();
        let Some(mut expected) = expected else {
            self.errors.push(ParseError {
                message: format!("Field {} not found in struct", name),
                span: field.span,
            });
            return;
        };

        if let Type::Generic(ref name) = expected {
            // if type is generic there should be something in the registry
            let current = generics_registry.types.get_mut(name).unwrap();
            if matches!(current, Type::Dynamic) {
                *current = value;
                return;
            }
            expected = current.clone();
        }

        if !value.is_assignable_to(&expected) {
            self.errors.push(ParseError {
                message: format!("Key type mismatch: expected {}, found {}", expected, value),
                span: field.span,
            });
        }
    }

    /// Push an error for every built field not found in the struct definition
    fn report_unknown_fields(
        &mut self,
        fields: &Vec<Spanned<FieldAssignment>>,
        expected: &HashMap<String, Type>,
    ) {
        for field in fields {
            if !expected.contains_key(&field.node.name) {
                self.errors.push(ParseError {
                    message: "No such field found".to_string(),
                    span: field.span,
                });
            }
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
