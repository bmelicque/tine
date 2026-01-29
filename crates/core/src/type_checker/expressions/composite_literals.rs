use core::panic;
use std::collections::HashMap;

use crate::{
    ast::{self, StructLiteralField},
    diagnostics::DiagnosticKind,
    type_checker::analysis_context::type_store::TypeStore,
    types::{
        self, ArrayType, MapType, OptionType, StructField, StructType, TupleType, Type, TypeId,
    },
};

use super::super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_composite_literal(&mut self, node: &ast::CompositeLiteral) -> TypeId {
        match node {
            ast::CompositeLiteral::AnonymousStruct(node) => {
                self.error(DiagnosticKind::MissingConstructorName, node.loc.decrement());
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
            let error = DiagnosticKind::UnexpectedStruct {
                expected: self.session.display_type(expected_type),
            };
            self.error(error, node.loc);
            return TypeStore::UNKNOWN;
        };
        let ty = self.intern_unique(Type::Struct(StructType { id: st.id, fields }));
        self.ctx.save_expression_type(node.loc, ty)
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
                self.check_assigned_type(expected_element_type, element_type, element.loc());
            }
        }

        if self.resolve(expected_element_type).is_unresolved() {
            self.error(DiagnosticKind::CannotInferType, node.loc);
            expected_element_type = TypeStore::UNKNOWN;
        }

        let ty = self.intern(Type::Array(ArrayType {
            element: expected_element_type,
        }));
        self.ctx.save_expression_type(node.loc, ty)
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
                self.check_assigned_type(key, entry_key, entry.key.loc());
            }

            if self.resolve(value).is_unresolved() {
                value = entry_value
            } else {
                self.check_assigned_type(value, entry_value, entry.value.loc());
            }
        }

        if self.resolve(key).is_unresolved() {
            self.error(DiagnosticKind::CannotInferType, node.loc);
            key = TypeStore::UNKNOWN;
        }
        if self.resolve(value).is_unresolved() {
            self.error(DiagnosticKind::CannotInferType, node.loc);
            value = TypeStore::UNKNOWN;
        }

        let ty = self.intern(Type::Map(MapType { key, value }));
        self.ctx.save_expression_type(node.loc, ty)
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
                self.check_assigned_type(some, value_ty, value.loc());
            }
        }

        if self.resolve(some).is_unresolved() {
            self.error(DiagnosticKind::CannotInferType, node.loc);
            some = TypeStore::UNKNOWN;
        }

        let ty = self.intern(Type::Option(OptionType { some }));
        self.ctx.save_expression_type(node.loc, ty)
    }

    pub fn visit_struct_literal(&mut self, node: &ast::StructLiteral) -> TypeId {
        let ty = self.visit_named_type(&node.ty);

        let st = match self.resolve(ty).clone() {
            Type::Struct(st) => st,
            Type::Unknown => {
                node.fields.iter().for_each(|f| {
                    self.visit_expression(&f.value);
                });
                return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
            }
            _ => panic!("Expected a named type"),
        };
        let st_id = st.id;
        let fields = self.visit_struct_body(&node.fields, st);
        let ty = self.intern_unique(Type::Struct(StructType { id: st_id, fields }));
        self.ctx.save_expression_type(node.loc, ty)
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
                let error = DiagnosticKind::UnknownMember {
                    member: name.clone(),
                };
                self.error(error, field.loc);
                continue;
            };
            if self.resolve(expected).is_unresolved() {
                expected = match substitutions.get(&expected) {
                    Some(e) => {
                        self.check_assigned_type(*e, value, field.value.loc());
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
                return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
            }
            _ => {
                let error = DiagnosticKind::ExpectedEnum {
                    got: node.ty.name.clone(),
                };
                self.error(error, node.ty.loc);
                return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
            }
        };

        let Some(variant) = enum_type
            .variants
            .iter()
            .find(|variant| variant.name == node.name)
        else {
            let error = DiagnosticKind::UnknownVariant {
                variant: node.name.clone(),
                enum_name: self.session.display_type(ty),
            };
            self.error(error, node.loc);
            return self.ctx.save_expression_type(node.loc, TypeStore::UNKNOWN);
        };
        match &node.body {
            Some(body) => self.visit_variant_body(body, variant.def),
            None => {
                if variant.def != TypeStore::UNIT && variant.def != TypeStore::UNKNOWN {
                    self.error(DiagnosticKind::InvalidVariantKind, node.loc);
                    TypeStore::UNKNOWN
                } else {
                    TypeStore::UNIT
                }
            }
        };

        self.ctx.save_expression_type(node.loc, ty)
    }

    fn visit_variant_body(&mut self, body: &ast::VariantLiteralBody, expected: TypeId) -> TypeId {
        match body {
            ast::VariantLiteralBody::Tuple(_) => self.visit_variant_tuple_body(body, expected),
            ast::VariantLiteralBody::Struct(fields) => {
                let Type::Struct(st) = self.resolve(expected).clone() else {
                    self.error(DiagnosticKind::InvalidVariantKind, body.loc());
                    return TypeStore::UNKNOWN;
                };
                let id = st.id;
                let fields = self.visit_struct_body(fields, st);
                self.intern_unique(Type::Struct(StructType { id, fields }))
            }
        }
    }

    fn visit_variant_tuple_body(
        &mut self,
        got: &ast::VariantLiteralBody,
        expected: TypeId,
    ) -> TypeId {
        let span = got.loc();
        let ast::VariantLiteralBody::Tuple(got_tuple) = got else {
            panic!()
        };
        let Type::Tuple(expected_tuple) = self.resolve(expected).clone() else {
            self.error(DiagnosticKind::InvalidVariantKind, span);
            return TypeStore::UNKNOWN;
        };
        if got_tuple.len() != expected_tuple.elements.len() {
            let error = DiagnosticKind::TupleElementCountMismatch {
                expected: expected_tuple.elements.len(),
                got: got_tuple.len(),
            };
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
                        self.check_assigned_type(*e, got_type, got.loc());
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

        self.intern(Type::Tuple(TupleType { elements }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::session::Session;
    use crate::types::{StructField, Type, Variant};
    use crate::{ast, Location, SymbolData, SymbolKind};

    #[test]
    fn test_visit_anonymous_struct_literal() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let user_type = checker.intern(Type::Struct(StructType {
            id: 0,
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    def: TypeStore::STRING,
                    optional: false,
                },
                StructField {
                    name: "age".to_string(),
                    def: TypeStore::INTEGER,
                    optional: false,
                },
            ],
        }));
        let struct_literal = ast::AnonymousStructLiteral {
            fields: vec![
                ast::StructLiteralField {
                    prop: "name".to_string(),
                    value: ast::Expression::StringLiteral(ast::StringLiteral {
                        loc: Location::dummy(),
                        text: "John".into(),
                    }),
                    loc: Location::dummy(),
                },
                ast::StructLiteralField {
                    prop: "age".to_string(),
                    value: ast::Expression::IntLiteral(ast::IntLiteral {
                        value: 30,
                        loc: Location::dummy(),
                    }),
                    loc: Location::dummy(),
                },
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_anonymous_struct_literal(&struct_literal, user_type);
        let result = checker.resolve(result).clone();
        assert!(matches!(result, Type::Struct(StructType { .. })));
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_array_literal() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let array_literal = ast::ArrayLiteral {
            ty: ast::ArrayType {
                element: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "int".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }))),
                loc: Location::dummy(),
            },
            elements: vec![
                ast::ExpressionOrAnonymous::Expression(ast::Expression::IntLiteral(
                    ast::IntLiteral {
                        value: 1,
                        loc: Location::dummy(),
                    },
                )),
                ast::ExpressionOrAnonymous::Expression(ast::Expression::IntLiteral(
                    ast::IntLiteral {
                        value: 2,
                        loc: Location::dummy(),
                    },
                )),
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_array_literal(&array_literal);
        let result = checker.resolve(result).clone();
        assert!(matches!(
            result,
            Type::Array(ArrayType {
                element: TypeStore::INTEGER
            })
        ));
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_map_literal() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let map_literal = ast::MapLiteral {
            ty: ast::MapType {
                key: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "str".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }))),
                value: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "int".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }))),
                loc: Location::dummy(),
            },
            entries: vec![
                ast::MapEntry {
                    key: Box::new(ast::Expression::StringLiteral(ast::StringLiteral {
                        loc: Location::dummy(),
                        text: "key1".into(),
                    })),
                    value: Box::new(ast::ExpressionOrAnonymous::Expression(
                        ast::Expression::IntLiteral(ast::IntLiteral {
                            value: 42,
                            loc: Location::dummy(),
                        }),
                    )),
                    loc: Location::dummy(),
                },
                ast::MapEntry {
                    key: Box::new(ast::Expression::StringLiteral(ast::StringLiteral {
                        loc: Location::dummy(),
                        text: "key1".into(),
                    })),
                    value: Box::new(ast::ExpressionOrAnonymous::Expression(
                        ast::Expression::IntLiteral(ast::IntLiteral {
                            value: 99,
                            loc: Location::dummy(),
                        }),
                    )),
                    loc: Location::dummy(),
                },
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_map_literal(&map_literal);
        let result = checker.resolve(result).clone();
        assert!(matches!(
            result,
            Type::Map(MapType {
                key: TypeStore::STRING,
                value: TypeStore::INTEGER
            })
        ));
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_option_literal() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        let option_literal = ast::OptionLiteral {
            ty: ast::OptionType {
                base: Some(Box::new(ast::Type::Named(ast::NamedType {
                    name: "int".to_string(),
                    args: None,
                    loc: Location::dummy(),
                }))),
                loc: Location::dummy(),
            },
            value: Some(Box::new(ast::ExpressionOrAnonymous::Expression(
                ast::Expression::IntLiteral(ast::IntLiteral {
                    value: 42,
                    loc: Location::dummy(),
                }),
            ))),
            loc: Location::dummy(),
        };

        let result = checker.visit_option_literal(&option_literal);
        let result = checker.resolve(result).clone();
        assert!(matches!(
            result,
            Type::Option(OptionType {
                some: TypeStore::INTEGER
            })
        ));
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_struct_literal() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        // Define the User struct type properly
        let user_type = types::Type::Struct(types::StructType {
            id: 0,
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    def: TypeStore::STRING,
                    optional: false,
                },
                StructField {
                    name: "age".to_string(),
                    def: TypeStore::INTEGER,
                    optional: false,
                },
            ],
        });

        let user_type_id = checker.intern_unique(user_type);
        checker.ctx.register_symbol(SymbolData {
            name: "User".into(),
            ty: user_type_id,
            kind: SymbolKind::Type { members: vec![] },
            ..Default::default()
        });

        let struct_literal = ast::StructLiteral {
            ty: ast::NamedType {
                name: "User".to_string(),
                args: None,
                loc: Location::dummy(),
            },
            fields: vec![
                ast::StructLiteralField {
                    prop: "name".to_string(),
                    value: ast::Expression::StringLiteral(ast::StringLiteral {
                        loc: Location::dummy(),
                        text: "John".into(),
                    }),
                    loc: Location::dummy(),
                },
                ast::StructLiteralField {
                    prop: "age".to_string(),
                    value: ast::Expression::IntLiteral(ast::IntLiteral {
                        value: 30,
                        loc: Location::dummy(),
                    }),
                    loc: Location::dummy(),
                },
            ],
            loc: Location::dummy(),
        };

        let result = checker.visit_struct_literal(&struct_literal);
        let result_resolved = checker.resolve(result);

        assert!(matches!(
            result_resolved,
            types::Type::Struct(types::StructType { .. })
        ));
        assert!(checker.diagnostics.is_empty());
    }

    #[test]
    fn test_visit_variant_literal_valid() {
        let session = Session::new();
        let mut checker = TypeChecker::new(&session, 0);
        // Define a sum type with variants
        let enum_type = types::Type::Enum(types::EnumType {
            id: 0,
            variants: vec![
                Variant {
                    name: "Circle".to_string(),
                    def: checker.intern_unique(types::Type::Struct(types::StructType {
                        id: 0,
                        fields: vec![StructField {
                            name: "radius".to_string(),
                            def: TypeStore::INTEGER,
                            optional: false,
                        }],
                    })),
                },
                Variant {
                    name: "Rectangle".to_string(),
                    def: checker.intern_unique(types::Type::Struct(types::StructType {
                        id: 0,
                        fields: vec![
                            StructField {
                                name: "width".to_string(),
                                def: TypeStore::INTEGER,
                                optional: false,
                            },
                            StructField {
                                name: "height".to_string(),
                                def: TypeStore::INTEGER,
                                optional: false,
                            },
                        ],
                    })),
                },
            ],
        });

        let shape_type_id = checker.intern_unique(enum_type);
        checker.ctx.register_symbol(SymbolData {
            name: "Shape".into(),
            ty: shape_type_id,
            kind: SymbolKind::Type { members: vec![] },
            ..Default::default()
        });

        // Create a valid variant literal
        let variant_literal = ast::VariantLiteral {
            ty: ast::NamedType {
                name: "Shape".to_string(),
                args: None,
                loc: Location::dummy(),
            },
            name: "Circle".to_string(),
            body: Some(ast::VariantLiteralBody::Struct(vec![
                ast::StructLiteralField {
                    prop: "radius".to_string(),
                    value: ast::Expression::IntLiteral(ast::IntLiteral {
                        value: 10,
                        loc: Location::dummy(),
                    }),
                    loc: Location::dummy(),
                },
            ])),
            loc: Location::dummy(),
        };

        let result = checker.visit_variant_literal(&variant_literal);
        let result = checker.resolve(result).clone();
        assert!(
            matches!(result, types::Type::Enum(types::EnumType { .. })),
            "expected enum type, got {:?}",
            result
        );
        assert!(
            checker.diagnostics.is_empty(),
            "expected no errors, got {:?}",
            checker.diagnostics
        );
    }
}
