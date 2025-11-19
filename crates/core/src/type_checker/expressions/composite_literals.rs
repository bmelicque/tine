use core::panic;
use std::collections::HashMap;

use crate::{
    ast::{self, StructLiteralField},
    type_checker::analysis_context::type_store::TypeStore,
    types::{
        self, ArrayType, MapType, OptionType, StructField, StructType, TupleType, Type, TypeId,
    },
};

use super::super::TypeChecker;

impl TypeChecker {
    pub fn visit_composite_literal(&mut self, node: &ast::CompositeLiteral) -> TypeId {
        match node {
            ast::CompositeLiteral::AnonymousStruct(node) => {
                self.error(
                    "missing type constructor in composite literal".into(),
                    node.span,
                );
                TypeStore::UNKNOWN
            }
            ast::CompositeLiteral::Array(node) => self.visit_array_literal(node),
            ast::CompositeLiteral::Map(node) => self.visit_map_literal(node),
            ast::CompositeLiteral::Option(node) => self.visit_option_literal(node),
            ast::CompositeLiteral::Struct(node) => self.visit_struct_literal(node),
            ast::CompositeLiteral::Variant(node) => self.visit_variant_literal(node),
        }
    }

    pub fn visit_anonymous_struct_literal(
        &mut self,
        node: &ast::AnonymousStructLiteral,
        expected_type: TypeId,
    ) -> TypeId {
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
        let Type::Struct(st) = self.resolve(expected_type) else {
            self.error(format!("type mismatch"), node.span);
            return TypeStore::UNKNOWN;
        };
        let ty = self
            .analysis_context
            .type_store
            .add(Type::Struct(StructType { id: st.id, fields }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    pub fn visit_array_literal(&mut self, node: &ast::ArrayLiteral) -> TypeId {
        let ty = self.visit_array_type(&node.ty);
        let Type::Array(array) = self.resolve(ty) else {
            panic!("Expected an array type");
        };
        let mut expected_element_type = array.element;

        for element in &node.elements {
            let element_type = self.visit_expression_or_anonymous(&element, expected_element_type);
            if self.resolve(expected_element_type).is_unresolved() {
                expected_element_type = element_type
            } else {
                self.check_assigned_type(expected_element_type, element_type, element.as_span());
            }
        }

        if self.resolve(expected_element_type).is_unresolved() {
            self.error("cannot infer element type".to_string(), node.span);
            expected_element_type = TypeStore::UNKNOWN;
        }

        let ty = self.analysis_context.type_store.add(Type::Array(ArrayType {
            element: expected_element_type,
        }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    pub fn visit_map_literal(&mut self, node: &ast::MapLiteral) -> TypeId {
        let ty = self.visit_map_type(&node.ty);
        let Type::Map(map) = self.resolve(ty) else {
            panic!("Expected a map type");
        };
        let mut key = map.key;
        let mut value = map.value;

        for entry in node.entries.iter() {
            let entry_key = self.visit_expression(&entry.key);
            let entry_value = self.visit_expression_or_anonymous(&entry.value, value);

            if self.resolve(key).is_unresolved() {
                key = entry_key
            } else {
                self.check_assigned_type(key, entry_key, entry.key.as_span());
            }

            if self.resolve(value).is_unresolved() {
                value = entry_value
            } else {
                self.check_assigned_type(value, entry_value, entry.value.as_span());
            }
        }

        if self.resolve(key).is_unresolved() {
            self.error("cannot infer key type".to_string(), node.span);
            key = TypeStore::UNKNOWN;
        }
        if self.resolve(value).is_unresolved() {
            self.error("cannot infer value type".to_string(), node.span);
            value = TypeStore::UNKNOWN;
        }

        let ty = self
            .analysis_context
            .type_store
            .add(Type::Map(MapType { key, value }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    pub fn visit_option_literal(&mut self, node: &ast::OptionLiteral) -> TypeId {
        let ty = self.visit_option_type(&node.ty);
        let Type::Option(option) = self.resolve(ty) else {
            panic!("Expected an option type");
        };
        let mut some = option.some;

        if let Some(ref value) = node.value {
            let value_ty = self.visit_expression_or_anonymous(value, some);
            if self.resolve(some).is_unresolved() {
                some = value_ty
            } else {
                self.check_assigned_type(some, value_ty, value.as_span());
            }
        }

        if self.resolve(some).is_unresolved() {
            self.error("Cannot infer type".to_string(), node.span);
            some = TypeStore::UNKNOWN;
        }

        let ty = self
            .analysis_context
            .type_store
            .add(Type::Option(OptionType { some }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    pub fn visit_struct_literal(&mut self, node: &ast::StructLiteral) -> TypeId {
        let ty = self.visit_named_type(&node.ty);

        let st = match self.resolve(ty).clone() {
            Type::Struct(st) => st,
            Type::Unknown => {
                node.fields.iter().for_each(|f| {
                    self.visit_expression(&f.value);
                });
                return self
                    .analysis_context
                    .save_expression_type(node.span, TypeStore::UNKNOWN);
            }
            _ => panic!("Expected a named type"),
        };
        let st_id = st.id;
        let fields = self.visit_struct_body(&node.fields, st);
        let ty = self
            .analysis_context
            .type_store
            .add(Type::Struct(StructType { id: st_id, fields }));
        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_struct_body(
        &mut self,
        got_fields: &Vec<StructLiteralField>,
        expected: StructType,
    ) -> Vec<StructField> {
        let mut fields: Vec<StructField> = Vec::with_capacity(got_fields.len());
        let mut substitutions = HashMap::new();
        for field in got_fields {
            let name = &field.prop;
            let value = self.visit_expression(&field.value);
            let expected_field = expected.fields.iter().find(|field| field.name == *name);
            let expected = expected_field.map(|field| field.def);
            let Some(mut expected) = expected else {
                self.error(
                    format!("property '{}' does not exist in this type", name),
                    field.span,
                );
                continue;
            };
            if self.resolve(expected).is_unresolved() {
                expected = match substitutions.get(&expected) {
                    Some(e) => {
                        self.check_assigned_type(*e, value, field.value.as_span());
                        *e
                    }
                    None => {
                        substitutions.insert(expected, value);
                        value
                    }
                }
            }
            fields.push(StructField {
                name: name.clone(),
                def: expected,
                optional: expected_field.map(|f| f.optional).unwrap_or(false),
            });
        }
        fields
    }

    fn visit_variant_literal(&mut self, node: &ast::VariantLiteral) -> TypeId {
        let ty = self.visit_named_type(&node.ty);
        let enum_type = match self.resolve(ty).clone() {
            Type::Enum(e) => e,
            Type::Unknown => {
                if let Some(body) = &node.body {
                    self.visit_variant_body(body, TypeStore::UNKNOWN);
                }
                return self
                    .analysis_context
                    .save_expression_type(node.span, TypeStore::UNKNOWN);
            }
            _ => {
                self.error(
                    format!("enum expected, got '{}'", node.ty.name),
                    node.ty.span,
                );
                return self
                    .analysis_context
                    .save_expression_type(node.span, TypeStore::UNKNOWN);
            }
        };

        let Some(variant) = enum_type
            .variants
            .iter()
            .find(|variant| variant.name == node.name)
        else {
            self.error(
                format!("Variant '{}' does not exist on type {}", node.name, ty),
                node.span,
            );
            return self
                .analysis_context
                .save_expression_type(node.span, TypeStore::UNKNOWN);
        };
        match &node.body {
            Some(body) => self.visit_variant_body(body, variant.def),
            None => {
                if variant.def != TypeStore::UNIT && variant.def != TypeStore::UNKNOWN {
                    self.error(
                        "expected structured or tuple variant, got unit".into(),
                        node.span,
                    );
                    TypeStore::UNKNOWN
                } else {
                    TypeStore::UNIT
                }
            }
        };

        self.analysis_context.save_expression_type(node.span, ty)
    }

    fn visit_variant_body(&mut self, body: &ast::VariantLiteralBody, expected: TypeId) -> TypeId {
        match body {
            ast::VariantLiteralBody::Tuple(_) => self.visit_variant_tuple_body(body, expected),
            ast::VariantLiteralBody::Struct(fields) => {
                let Type::Struct(st) = self.resolve(expected).clone() else {
                    self.error(
                        "expected tuple or unit variant, got a structured type".into(),
                        body.as_span(),
                    );
                    return TypeStore::UNKNOWN;
                };
                let id = st.id;
                let fields = self.visit_struct_body(fields, st);
                self.analysis_context
                    .type_store
                    .add(Type::Struct(StructType { id, fields }))
            }
        }
    }

    fn visit_variant_tuple_body(
        &mut self,
        got: &ast::VariantLiteralBody,
        expected: TypeId,
    ) -> TypeId {
        let span = got.as_span();
        let ast::VariantLiteralBody::Tuple(got_tuple) = got else {
            panic!()
        };
        let Type::Tuple(expected_tuple) = self.resolve(expected).clone() else {
            self.error(
                "expected structured or unit variant, got a tuple".into(),
                span,
            );
            return TypeStore::UNKNOWN;
        };
        if got_tuple.len() != expected_tuple.elements.len() {
            let error = format!(
                "expected {} element(s), got {}",
                expected_tuple.elements.len(),
                got_tuple.len()
            );
            self.error(error, span);
            return TypeStore::UNKNOWN;
        }

        let mut elements: Vec<TypeId> = Vec::with_capacity(got_tuple.len());
        let mut substitutions = HashMap::new();
        for (i, got) in got_tuple.iter().enumerate() {
            let got_type = self.visit_expression_or_anonymous(got, expected_tuple.elements[i]);
            let mut expected_type = expected_tuple.elements[i];
            if self.resolve(expected_tuple.elements[i]).is_unresolved() {
                expected_type = match substitutions.get(&expected_type) {
                    Some(e) => {
                        self.check_assigned_type(*e, got_type, got.as_span());
                        *e
                    }
                    None => {
                        substitutions.insert(expected_type, got_type);
                        got_type
                    }
                }
            }
            elements.push(expected_type);
        }

        self.analysis_context
            .type_store
            .add(Type::Tuple(TupleType { elements }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{StructField, Type, Variant};
    use crate::{ast, SymbolData};

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("_", 0, 0).unwrap()
    }

    #[test]
    fn test_visit_anonymous_struct_literal() {
        let mut checker = TypeChecker::new(Vec::new());
        let user_type = checker
            .analysis_context
            .type_store
            .add(Type::Struct(StructType {
                id: checker.analysis_context.type_store.get_next_id(),
                fields: vec![
                    StructField {
                        name: "name".to_string(),
                        def: TypeStore::STRING,
                        optional: false,
                    },
                    StructField {
                        name: "age".to_string(),
                        def: TypeStore::NUMBER,
                        optional: false,
                    },
                ],
            }));
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

        let result = checker.visit_anonymous_struct_literal(&struct_literal, user_type);
        let result = checker.resolve(result).clone();
        assert!(matches!(result, Type::Struct(StructType { .. })));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_array_literal() {
        let mut checker = TypeChecker::new(Vec::new());
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
        let result = checker.resolve(result).clone();
        assert!(matches!(
            result,
            Type::Array(ArrayType {
                element: TypeStore::NUMBER
            })
        ));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_map_literal() {
        let mut checker = TypeChecker::new(Vec::new());
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
        let result = checker.resolve(result).clone();
        assert!(matches!(
            result,
            Type::Map(MapType {
                key: TypeStore::STRING,
                value: TypeStore::NUMBER
            })
        ));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_option_literal() {
        let mut checker = TypeChecker::new(Vec::new());
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
        let result = checker.resolve(result).clone();
        assert!(matches!(
            result,
            Type::Option(OptionType {
                some: TypeStore::NUMBER
            })
        ));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_struct_literal() {
        let mut checker = TypeChecker::new(Vec::new());

        // Define the User struct type properly
        let user_type = types::Type::Struct(types::StructType {
            id: checker.analysis_context.type_store.get_next_id(),
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    def: TypeStore::STRING,
                    optional: false,
                },
                StructField {
                    name: "age".to_string(),
                    def: TypeStore::NUMBER,
                    optional: false,
                },
            ],
        });

        let user_type_id = checker.analysis_context.type_store.add(user_type);
        checker
            .analysis_context
            .register_symbol(SymbolData::new_type(
                "User".into(),
                user_type_id,
                dummy_span(),
            ));

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
        let result_resolved = checker.resolve(result);

        assert!(matches!(
            *result_resolved,
            types::Type::Struct(types::StructType { .. })
        ));
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_variant_literal_valid() {
        let mut checker = TypeChecker::new(Vec::new());

        // Define a sum type with variants
        let enum_type = types::Type::Enum(types::EnumType {
            id: checker.analysis_context.type_store.get_next_id(),
            variants: vec![
                Variant {
                    name: "Circle".to_string(),
                    def: checker.analysis_context.type_store.add(types::Type::Struct(
                        types::StructType {
                            id: checker.analysis_context.type_store.get_next_id(),
                            fields: vec![StructField {
                                name: "radius".to_string(),
                                def: TypeStore::NUMBER,
                                optional: false,
                            }],
                        },
                    )),
                },
                Variant {
                    name: "Rectangle".to_string(),
                    def: checker.analysis_context.type_store.add(types::Type::Struct(
                        types::StructType {
                            id: checker.analysis_context.type_store.get_next_id(),
                            fields: vec![
                                StructField {
                                    name: "width".to_string(),
                                    def: TypeStore::NUMBER,
                                    optional: false,
                                },
                                StructField {
                                    name: "height".to_string(),
                                    def: TypeStore::NUMBER,
                                    optional: false,
                                },
                            ],
                        },
                    )),
                },
            ],
        });

        let shape_type_id = checker.analysis_context.type_store.add(enum_type);
        checker
            .analysis_context
            .register_symbol(SymbolData::new_type(
                "Shape".into(),
                shape_type_id,
                dummy_span(),
            ));

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
        let result = checker.resolve(result).clone();
        assert!(
            matches!(result, types::Type::Enum(types::EnumType { .. })),
            "expected enum type, got {:?}",
            result
        );
        assert!(checker.errors.is_empty());
    }
}
