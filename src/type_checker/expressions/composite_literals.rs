use core::panic;
use std::collections::HashMap;

use crate::{ast, parser::parser::ParseError, types};

use super::super::{scopes::TypeRegistry, TypeChecker};

impl TypeChecker {
    pub fn visit_composite_literal(&mut self, node: &ast::CompositeLiteral) -> types::Type {
        match node {
            ast::CompositeLiteral::AnonymousStruct(node) => {
                self.visit_anonymous_struct_literal(node).into()
            }
            ast::CompositeLiteral::Array(node) => self.visit_array_literal(node).into(),
            ast::CompositeLiteral::Map(node) => self.visit_map_literal(node).into(),
            ast::CompositeLiteral::Option(node) => self.visit_option_literal(node).into(),
            ast::CompositeLiteral::Struct(node) => self.visit_struct_literal(node),
            ast::CompositeLiteral::Variant(node) => self.visit_variant_literal(node),
        }
    }

    pub fn visit_anonymous_struct_literal(
        &mut self,
        node: &ast::AnonymousStructLiteral,
    ) -> types::StructType {
        let fields = node
            .fields
            .iter()
            .map(|field| {
                let name = field.prop.clone();
                let def = self.visit_expression(&field.value);
                types::StructField {
                    name,
                    def,
                    optional: true,
                }
            })
            .collect();
        self.set_type_at(node.span, types::StructType { fields })
    }

    pub fn visit_array_literal(&mut self, node: &ast::ArrayLiteral) -> types::ArrayType {
        let ty = self.visit_array_type(&node.ty);
        let types::Type::Array(array) = ty else {
            panic!("Expected an array type");
        };
        let mut element = array.element;

        node.elements
            .iter()
            .for_each(|el| self.visit_array_literal_element(el, &mut element));

        if matches!(element.as_ref(), types::Type::Dynamic) {
            self.errors.push(ParseError {
                message: "Cannot infer type".to_string(),
                span: node.span,
            });
            element = Box::new(types::Type::Unknown);
        }

        self.set_type_at(node.span, types::ArrayType { element })
    }

    fn visit_array_literal_element(
        &mut self,
        node: &ast::ExpressionOrAnonymous,
        inner: &mut types::Type,
    ) {
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

    pub fn visit_map_literal(&mut self, node: &ast::MapLiteral) -> types::MapType {
        let ty = self.visit_map_type(&node.ty);
        let types::Type::Map(map) = ty else {
            panic!("Expected a map type");
        };
        let types::MapType { mut key, mut value } = map;

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

        self.set_type_at(node.span, types::MapType { key, value })
    }

    pub fn visit_option_literal(&mut self, node: &ast::OptionLiteral) -> types::OptionType {
        let ty = self.visit_option_type(&node.ty);
        let types::Type::Option(option) = ty else {
            panic!("Expected an option type");
        };
        let mut some = option.some;

        if let Some(ref value) = node.value {
            let value_ty = self.visit_expression_or_anonymous(value);
            match self.check_dynamic_type(&value_ty, &some) {
                Ok(ty) => some = ty,
                Err(message) => {
                    self.errors.push(ParseError {
                        message,
                        span: value.as_span(),
                    });
                }
            }
        }

        if matches!(some.as_ref(), types::Type::Dynamic) {
            self.errors.push(ParseError {
                message: "Cannot infer type".to_string(),
                span: node.span,
            });
            some = Box::new(types::Type::Unknown);
        }

        self.set_type_at(node.span, types::OptionType { some })
    }

    pub fn visit_struct_literal(&mut self, node: &ast::StructLiteral) -> types::Type {
        let ty = self.visit_named_type(&node.ty);
        let types::Type::Named(ref named) = ty else {
            panic!("Expected a named type");
        };

        let types::Type::Struct(st) = self.type_registry.lookup(&named.name).unwrap() else {
            self.errors.push(ParseError {
                message: format!("Expected a structured type, found {:?}", ty),
                span: node.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
        };

        // setup generics registry
        let names = self.type_registry.get_type_params(&named.name);
        let mut generics_registry = TypeRegistry::create(&names, &named.args);

        self.visit_struct_body(st.fields, &node.fields, &mut generics_registry);

        // TODO: adjust type using inferred generics
        self.set_type_at(node.span, ty)
    }

    fn visit_struct_body(
        &mut self,
        expected: Vec<types::StructField>,
        got: &Vec<ast::StructLiteralField>,
        registry: &mut TypeRegistry,
    ) {
        // set up map of expected field types
        let mut field_map = HashMap::new();
        for field in expected.iter() {
            field_map.insert(&field.name, field.def.clone());
        }

        got.iter()
            .for_each(|field| self.visit_field_assignment(field, &field_map, registry));
    }

    fn visit_field_assignment(
        &mut self,
        field: &ast::StructLiteralField,
        expected_map: &HashMap<&String, types::Type>,
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

        if let types::Type::Generic(ref ty) = expected {
            // if type is generic there should be something in the registry
            let current = generics_registry.types.get_mut(&ty.name).unwrap();
            if matches!(current, types::Type::Dynamic) {
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

    fn visit_variant_literal(&mut self, node: &ast::VariantLiteral) -> types::Type {
        let ty = self.visit_named_type(&node.ty);
        let unwrapped = self.unwrap_named_type(&ty);
        let types::Type::Enum(enum_type) = unwrapped else {
            self.errors.push(ParseError {
                message: format!("Sum type expected, got {}", unwrapped),
                span: node.ty.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
        };
        let Some(variant) = enum_type
            .variants
            .iter()
            .find(|variant| variant.name == node.name)
        else {
            self.errors.push(ParseError {
                message: format!("Variant '{}' does not exist on type {}", node.name, ty),
                span: node.span,
            });
            return self.set_type_at(node.span, types::Type::Unknown);
        };
        if let Err(message) = self.visit_variant_body(&node.body, &variant.def) {
            self.errors.push(ParseError {
                message,
                span: node.span,
            })
        }

        self.set_type_at(node.span, ty)
    }

    fn visit_variant_body(
        &mut self,
        body: &Option<ast::VariantLiteralBody>,
        expected: &types::Type,
    ) -> Result<(), String> {
        let Some(body) = body else {
            return if *expected != types::Type::Unit {
                Err("Arguments expected".into())
            } else {
                Ok(())
            };
        };
        match body {
            ast::VariantLiteralBody::Tuple(body) => {
                let got: types::Type = types::TupleType {
                    elements: body
                        .iter()
                        .map(|el| self.visit_expression_or_anonymous(el))
                        .collect(),
                }
                .into();
                if got != *expected {
                    return Err("Invalid arguments".into());
                }
            }
            ast::VariantLiteralBody::Struct(body) => {
                let types::Type::Struct(st) = expected else {
                    return Err("Structured body exprected".into());
                };
                self.visit_struct_body(st.fields.clone(), &body, &mut TypeRegistry::new());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::types::{StructField, Type, Variant};

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("_", 0, 0).unwrap()
    }

    #[test]
    fn test_visit_anonymous_struct_literal() {
        let mut checker = TypeChecker::new();
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
                        value: ordered_float::OrderedFloat(30.0),
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
            types::StructType {
                fields: vec![
                    StructField {
                        name: "name".to_string(),
                        def: types::Type::String,
                        optional: true,
                    },
                    StructField {
                        name: "age".to_string(),
                        def: types::Type::Number,
                        optional: true,
                    },
                ],
            },
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_array_literal() {
        let mut checker = TypeChecker::new();
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
                        value: ordered_float::OrderedFloat(1.0),
                        span: dummy_span(),
                    },
                )),
                ast::ExpressionOrAnonymous::Expression(ast::Expression::NumberLiteral(
                    ast::NumberLiteral {
                        value: ordered_float::OrderedFloat(2.0),
                        span: dummy_span(),
                    },
                )),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_array_literal(&array_literal);
        assert_eq!(
            result,
            types::ArrayType {
                element: Box::new(Type::Number)
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_map_literal() {
        let mut checker = TypeChecker::new();
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
                            value: ordered_float::OrderedFloat(42.0),
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
                            value: ordered_float::OrderedFloat(99.0),
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
            types::MapType {
                key: Box::new(Type::String),
                value: Box::new(Type::Number),
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_option_literal() {
        let mut checker = TypeChecker::new();
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
                    value: ordered_float::OrderedFloat(42.0),
                    span: dummy_span(),
                }),
            ))),
            span: dummy_span(),
        };

        let result = checker.visit_option_literal(&option_literal);
        assert_eq!(
            result,
            types::OptionType {
                some: Box::new(Type::Number)
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_struct_literal() {
        let mut checker = TypeChecker::new();
        checker.type_registry.define(
            "User",
            types::Type::Struct(types::StructType {
                fields: vec![
                    StructField {
                        name: "name".to_string(),
                        def: types::Type::String,
                        optional: false,
                    },
                    StructField {
                        name: "age".to_string(),
                        def: types::Type::Number,
                        optional: false,
                    },
                ],
            }),
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
                        value: ordered_float::OrderedFloat(30.0),
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
            types::NamedType {
                name: "User".to_string(),
                args: vec![],
            }
            .into()
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_variant_literal_valid() {
        let mut checker = TypeChecker::new();

        // Define a sum type with variants
        checker.type_registry.define(
            "Shape",
            types::Type::Enum(types::EnumType {
                variants: vec![
                    Variant {
                        name: "Circle".to_string(),
                        def: types::StructType {
                            fields: vec![StructField {
                                name: "radius".to_string(),
                                def: types::Type::Number,
                                optional: false,
                            }],
                        }
                        .into(),
                    },
                    Variant {
                        name: "Rectangle".to_string(),
                        def: types::StructType {
                            fields: vec![
                                StructField {
                                    name: "width".to_string(),
                                    def: types::Type::Number,
                                    optional: false,
                                },
                                StructField {
                                    name: "height".to_string(),
                                    def: types::Type::Number,
                                    optional: false,
                                },
                            ],
                        }
                        .into(),
                    },
                ],
            }),
            None,
        );

        // Create a valid variant literal
        let variant_literal = ast::VariantLiteral {
            ty: ast::NamedType {
                name: "Shape".to_string(),
                args: None,
                span: dummy_span(),
            },
            name: "Circle".to_string(),
            body: Some(ast::VariantLiteralBody::Struct(vec![
                ast::StructLiteralField {
                    prop: "radius".to_string(),
                    value: ast::Expression::NumberLiteral(ast::NumberLiteral {
                        value: ordered_float::OrderedFloat(10.0),
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                },
            ])),
            span: dummy_span(),
        };

        let result = checker.visit_variant_literal(&variant_literal);
        assert_eq!(
            result,
            types::NamedType {
                name: "Shape".to_string(),
                args: vec![],
            }
            .into()
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_variant_literal_non_existent_variant() {
        let mut checker = TypeChecker::new();

        // Define a sum type with variants
        checker.type_registry.define(
            "Shape",
            types::Type::Enum(types::EnumType {
                variants: vec![Variant {
                    name: "Circle".to_string(),
                    def: types::Type::Struct(types::StructType {
                        fields: vec![StructField {
                            name: "radius".to_string(),
                            def: types::Type::Number,
                            optional: false,
                        }],
                    }),
                }],
            }),
            None,
        );

        // Create a variant literal with a non-existent variant
        let variant_literal = ast::VariantLiteral {
            ty: ast::NamedType {
                name: "Shape".to_string(),
                args: None,
                span: dummy_span(),
            },
            name: "Triangle".to_string(),
            body: None,
            span: dummy_span(),
        };

        let result = checker.visit_variant_literal(&variant_literal);
        assert_eq!(result, types::Type::Unknown);
        assert_eq!(checker.errors.len(), 1);
        assert!(checker.errors[0]
            .message
            .contains("Variant 'Triangle' does not exist on type Shape"));
    }

    #[test]
    fn test_visit_variant_literal_mismatched_body() {
        let mut checker = TypeChecker::new();

        // Define a sum type with variants
        checker.type_registry.define(
            "Shape",
            types::Type::Enum(types::EnumType {
                variants: vec![Variant {
                    name: "Circle".to_string(),
                    def: types::Type::Struct(types::StructType {
                        fields: vec![StructField {
                            name: "radius".to_string(),
                            def: types::Type::Number,
                            optional: false,
                        }],
                    }),
                }],
            }),
            None,
        );

        // Create a variant literal with a mismatched body
        let variant_literal = ast::VariantLiteral {
            ty: ast::NamedType {
                name: "Shape".to_string(),
                args: None,
                span: dummy_span(),
            },
            name: "Circle".to_string(),
            body: Some(ast::VariantLiteralBody::Struct(vec![
                ast::StructLiteralField {
                    prop: "diameter".to_string(),
                    value: ast::Expression::NumberLiteral(ast::NumberLiteral {
                        value: ordered_float::OrderedFloat(20.0),
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                },
            ])),
            span: dummy_span(),
        };

        let result = checker.visit_variant_literal(&variant_literal);
        assert_eq!(
            result,
            types::NamedType {
                name: "Shape".to_string(),
                args: vec![]
            }
            .into()
        );
        assert_eq!(checker.errors.len(), 1, "{:?}", checker.errors);
        assert!(checker.errors[0]
            .message
            .contains("Field diameter not found in struct"));
    }
}
