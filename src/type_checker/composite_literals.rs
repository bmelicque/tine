use core::panic;
use std::collections::HashMap;

use crate::{
    ast::{self, StructLiteralField},
    parser::parser::ParseError,
    types::{StructField, Type},
};

use super::{scopes::TypeRegistry, TypeChecker};

impl TypeChecker {
    pub fn visit_composite_literal(&mut self, node: &ast::CompositeLiteral) -> Type {
        match node {
            ast::CompositeLiteral::AnonymousArray(node) => self.visit_anonymous_array_literal(node),
            ast::CompositeLiteral::AnonymousStruct(node) => {
                self.visit_anonymous_struct_literal(node)
            }
            ast::CompositeLiteral::Array(node) => self.visit_array_literal(node),
            ast::CompositeLiteral::Map(node) => self.visit_map_literal(node),
            ast::CompositeLiteral::Option(node) => self.visit_option_literal(node),
            ast::CompositeLiteral::Struct(node) => self.visit_struct_literal(node),
        }
    }

    pub fn visit_anonymous_array_literal(&mut self, node: &ast::AnonymousArrayLiteral) -> Type {
        if node.elements.len() == 0 {
            return Type::Dynamic;
        }

        let mut ty = Type::Dynamic;
        for value in node.elements.iter() {
            let value_ty = self.visit_expression_or_anonymous(value);
            if ty == Type::Dynamic {
                ty = value_ty;
                continue;
            }
            if !value_ty.is_assignable_to(&ty) {
                self.errors.push(ParseError {
                    message: format!("Key type mismatch: expected {}, found {}", ty, value_ty),
                    span: value.as_span(),
                })
            }
        }

        Type::Array(Box::new(ty))
    }

    pub fn visit_anonymous_struct_literal(&mut self, node: &ast::AnonymousStructLiteral) -> Type {
        let fields = node
            .fields
            .iter()
            .map(|field| {
                let name = field.prop.clone();
                let def = self.visit_expression(&field.value);
                StructField {
                    name,
                    def,
                    optional: true,
                }
            })
            .collect();
        Type::Struct { fields }
    }

    pub fn visit_array_literal(&mut self, node: &ast::ArrayLiteral) -> Type {
        let ty = self.visit_array_type(&node.ty);
        let Type::Array(mut inner) = ty else {
            panic!("Expected an array type");
        };

        node.elements
            .iter()
            .for_each(|el| self.visit_array_literal_element(el, &mut inner));

        if matches!(inner.as_ref(), Type::Dynamic) {
            self.errors.push(ParseError {
                message: "Cannot infer type".to_string(),
                span: node.span,
            });
            inner = Box::new(Type::Unknown);
        }

        Type::Array(inner)
    }

    fn visit_array_literal_element(&mut self, node: &ast::ExpressionOrAnonymous, inner: &mut Type) {
        let value_ty = self.visit_expression_or_anonymous(node);
        match self.check_dynamic_type(&value_ty, inner) {
            Ok(ty) => *inner = (*ty).clone(),
            Err(message) => {
                self.errors.push(ParseError {
                    message,
                    span: node.as_span(),
                });
            }
        }
    }

    pub fn visit_map_literal(&mut self, node: &ast::MapLiteral) -> Type {
        let ty = self.visit_map_type(&node.ty);
        let Type::Map { mut key, mut value } = ty else {
            panic!("Expected a map type");
        };

        for entry in node.entries.iter() {
            let entry_key = self.visit_expression(&entry.key);
            let entry_value = self.visit_expression_or_anonymous(&entry.value);

            match self.check_dynamic_type(&entry_key, &key) {
                Ok(ty) => key = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: entry.span,
                    });
                }
            }

            match self.check_dynamic_type(&entry_value, &value) {
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

    pub fn visit_option_literal(&mut self, node: &ast::OptionLiteral) -> Type {
        let ty = self.visit_option_type(&node.ty);
        let Type::Option(mut inner) = ty else {
            panic!("Expected an option type");
        };

        if let Some(ref value) = node.value {
            let value_ty = self.visit_expression_or_anonymous(value);
            match self.check_dynamic_type(&value_ty, &inner) {
                Ok(ty) => inner = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: value.as_span(),
                    });
                }
            }
        }

        if matches!(inner.as_ref(), Type::Dynamic) {
            self.errors.push(ParseError {
                message: "Cannot infer type".to_string(),
                span: node.span,
            });
            inner = Box::new(Type::Unknown);
        }

        Type::Option(inner)
    }

    pub fn visit_struct_literal(&mut self, node: &ast::StructLiteral) -> Type {
        let ty = self.visit_named_type(&node.ty);
        let Type::Named { ref name, ref args } = ty else {
            panic!("Expected a named type");
        };

        let Type::Struct { fields } = self.type_registry.lookup(&name).unwrap() else {
            self.errors.push(ParseError {
                message: format!("Expected a structured type, found {:?}", ty),
                span: node.span,
            });
            return Type::Unknown;
        };

        // setup generics registry
        let names = self.type_registry.get_type_params(name);
        let mut generics_registry = TypeRegistry::create(&names, &args);

        // set up map of expected field types
        let mut field_map = HashMap::new();
        for field in fields.iter() {
            field_map.insert(&field.name, field.def.clone());
        }

        node.fields.iter().for_each(|field| {
            self.visit_field_assignment(field, &field_map, &mut generics_registry)
        });

        self.report_unknown_fields(&node.fields, &field_map);

        // TODO: adjust type using inferred generics
        ty
    }

    fn visit_field_assignment(
        &mut self,
        field: &StructLiteralField,
        expected_map: &HashMap<&String, Type>,
        generics_registry: &mut TypeRegistry,
    ) {
        let name = &field.prop;
        let value = self.visit_expression(&field.value);
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
        fields: &Vec<StructLiteralField>,
        expected: &HashMap<&String, Type>,
    ) {
        for field in fields {
            if !expected.contains_key(&field.prop) {
                self.errors.push(ParseError {
                    message: "No such field found".to_string(),
                    span: field.span,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::types::{StructField, Type};

    fn create_type_checker() -> TypeChecker {
        TypeChecker {
            errors: Vec::new(),
            symbols: Default::default(),
            type_registry: Default::default(),
        }
    }

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("_", 0, 0).unwrap()
    }

    #[test]
    fn test_visit_anonymous_array_literal() {
        let mut checker = create_type_checker();
        let array_literal = ast::AnonymousArrayLiteral {
            elements: vec![
                ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                    ast::NumberLiteral {
                        value: 1.0,
                        span: dummy_span(),
                    },
                )),
                ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                    ast::NumberLiteral {
                        value: 2.0,
                        span: dummy_span(),
                    },
                )),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_anonymous_array_literal(&array_literal);
        assert_eq!(result, Type::Array(Box::new(Type::Number)));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_anonymous_struct_literal() {
        let mut checker = create_type_checker();
        let struct_literal = ast::AnonymousStructLiteral {
            fields: vec![
                ast::StructLiteralField {
                    prop: "name".to_string(),
                    value: ast::Expression::StringLiteral(ast::StringLiteral {
                        span: pest::Span::new("John", 0, 4).unwrap(),
                    }),
                    span: dummy_span(),
                },
                ast::StructLiteralField {
                    prop: "age".to_string(),
                    value: ast::Expression::NumberLiteral(ast::NumberLiteral {
                        value: 30.0,
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                },
            ],
            span: dummy_span(),
        };

        let result = checker.visit_anonymous_struct_literal(&struct_literal);
        assert_eq!(
            result,
            Type::Struct {
                fields: vec![
                    StructField {
                        name: "name".to_string(),
                        def: Type::String,
                        optional: true,
                    },
                    StructField {
                        name: "age".to_string(),
                        def: Type::Number,
                        optional: true,
                    },
                ],
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_array_literal() {
        let mut checker = create_type_checker();
        let array_literal = ast::ArrayLiteral {
            ty: ast::ArrayType {
                element: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    span: dummy_span(),
                }))),
                span: dummy_span(),
            },
            elements: vec![
                ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                    ast::NumberLiteral {
                        value: 1.0,
                        span: dummy_span(),
                    },
                )),
                ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                    ast::NumberLiteral {
                        value: 2.0,
                        span: dummy_span(),
                    },
                )),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_array_literal(&array_literal);
        assert_eq!(result, Type::Array(Box::new(Type::Number)));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_map_literal() {
        let mut checker = create_type_checker();
        let map_literal = ast::MapLiteral {
            ty: ast::MapType {
                key: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "string".to_string(),
                    args: None,
                    span: dummy_span(),
                }))),
                value: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    span: dummy_span(),
                }))),
                span: dummy_span(),
            },
            entries: vec![
                ast::MapEntry {
                    key: Box::new(ast::Expression::StringLiteral(ast::StringLiteral {
                        span: pest::Span::new("key1", 0, 1).unwrap(),
                    })),
                    value: Box::new(ast::ExpressionOrAnonymous::Expression(
                        ast::Expression::NumberLiteral(ast::NumberLiteral {
                            value: 42.0,
                            span: dummy_span(),
                        }),
                    )),
                    span: dummy_span(),
                },
                ast::MapEntry {
                    key: Box::new(ast::Expression::StringLiteral(ast::StringLiteral {
                        span: pest::Span::new("key1", 0, 1).unwrap(),
                    })),
                    value: Box::new(ast::ExpressionOrAnonymous::Expression(
                        ast::Expression::NumberLiteral(ast::NumberLiteral {
                            value: 99.0,
                            span: dummy_span(),
                        }),
                    )),
                    span: dummy_span(),
                },
            ],
            span: dummy_span(),
        };

        let result = checker.visit_map_literal(&map_literal);
        assert_eq!(
            result,
            Type::Map {
                key: Box::new(Type::String),
                value: Box::new(Type::Number),
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_option_literal() {
        let mut checker = create_type_checker();
        let option_literal = ast::OptionLiteral {
            ty: ast::OptionType {
                base: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    span: dummy_span(),
                }))),
                span: dummy_span(),
            },
            value: Some(Box::new(ast::ExpressionOrAnonymous::Expression(
                ast::Expression::NumberLiteral(ast::NumberLiteral {
                    value: 42.0,
                    span: dummy_span(),
                }),
            ))),
            span: dummy_span(),
        };

        let result = checker.visit_option_literal(&option_literal);
        assert_eq!(result, Type::Option(Box::new(Type::Number)));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_struct_literal() {
        let mut checker = create_type_checker();
        checker.type_registry.define(
            "User",
            Type::Struct {
                fields: vec![
                    StructField {
                        name: "name".to_string(),
                        def: Type::String,
                        optional: false,
                    },
                    StructField {
                        name: "age".to_string(),
                        def: Type::Number,
                        optional: false,
                    },
                ],
            },
            None,
        );

        let struct_literal = ast::StructLiteral {
            ty: ast::NamedType {
                name: "User".to_string(),
                args: None,
                span: dummy_span(),
            },
            fields: vec![
                ast::StructLiteralField {
                    prop: "name".to_string(),
                    value: ast::Expression::StringLiteral(ast::StringLiteral {
                        span: pest::Span::new("John", 0, 1).unwrap(),
                    }),
                    span: dummy_span(),
                },
                ast::StructLiteralField {
                    prop: "age".to_string(),
                    value: ast::Expression::NumberLiteral(ast::NumberLiteral {
                        value: 30.0,
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                },
            ],
            span: dummy_span(),
        };

        let result = checker.visit_struct_literal(&struct_literal);
        assert_eq!(
            result,
            Type::Named {
                name: "User".to_string(),
                args: vec![],
            }
        );
        assert!(checker.errors.is_empty());
    }
}
