use crate::{ast, parser::parser::ParseError, types};

use super::{scopes::TypeMetadata, TypeChecker};

impl TypeChecker {
    pub fn visit_type_declaration(&mut self, node: &ast::TypeAlias) -> types::Type {
        if let Some(ref type_params) = node.params {
            for type_param in type_params {
                self.type_registry.define_generic(&type_param);
            }
        }
        let ty = self.visit_type_definition(&node.definition);
        self.type_registry.clear_generics();

        let name = &node.name;
        match self.type_registry.lookup(name) {
            Some(_) => self.errors.push(ParseError {
                message: format!("Type {} already defined", name),
                span: node.span,
            }),
            None => {
                let metadata = node.params.clone().map(|params| TypeMetadata {
                    type_params: params,
                });
                self.type_registry.define(&name, ty, metadata);
            }
        }

        types::Type::Void
    }

    fn visit_type_definition(&mut self, node: &ast::TypeDefinition) -> types::Type {
        match node {
            ast::TypeDefinition::Enum(e) => self.visit_enum_definition(e).into(),
            ast::TypeDefinition::Struct(s) => self.visit_struct_definition(s).into(),
            ast::TypeDefinition::Trait(t) => self.visit_trait_definition(t).into(),
            ast::TypeDefinition::Type(t) => self.visit_type(t),
        }
    }

    fn visit_enum_definition(&mut self, node: &ast::EnumDefinition) -> types::EnumType {
        let variants = node
            .variants
            .iter()
            .map(|variant| self.visit_variant_definition(variant))
            .collect();

        types::EnumType { variants }
    }

    fn visit_variant_definition(&mut self, node: &ast::VariantDefinition) -> types::Variant {
        let def: types::Type = match node {
            ast::VariantDefinition::Struct(s) => self.visit_struct_definition(&s.def).into(),
            ast::VariantDefinition::Tuple(t) => {
                let elements = t.elements.iter().map(|el| self.visit_type(el)).collect();
                types::TupleType { elements }.into()
            }
            ast::VariantDefinition::Unit(_) => types::Type::Unit,
        };
        types::Variant {
            name: node.as_name(),
            def,
        }
    }

    fn visit_struct_definition(&mut self, node: &ast::StructDefinition) -> types::StructType {
        let fields = node
            .fields
            .iter()
            .map(|field| self.visit_struct_definition_field(field))
            .collect();
        types::StructType { fields }
    }

    fn visit_struct_definition_field(
        &mut self,
        field: &ast::StructDefinitionField,
    ) -> types::StructField {
        let name = field.as_name();
        let def = match field {
            ast::StructDefinitionField::Mandatory(ref field) => self.visit_type(&field.definition),
            ast::StructDefinitionField::Optional(field) => self.visit_expression(&field.default),
        };
        types::StructField {
            name,
            def,
            optional: field.is_optional(),
        }
    }

    fn visit_trait_definition(&mut self, node: &ast::TraitDefinition) -> types::TraitType {
        self.type_registry.current_self = Some(node.name.clone());

        let method_types = node
            .body
            .fields
            .iter()
            .filter_map(|field| self.visit_trait_method_definition(field))
            .collect();

        self.type_registry.current_self = None;

        types::TraitType {
            methods: method_types,
        }
    }

    fn visit_trait_method_definition(
        &mut self,
        node: &ast::StructDefinitionField,
    ) -> Option<types::TraitMethod> {
        let as_field = self.visit_struct_definition_field(node);
        if !matches!(as_field.def, types::Type::Function { .. }) {
            self.errors.push(ParseError {
                message: format!(
                    "Only methods are allowed in trait definitions, found {}",
                    as_field.def
                ),
                span: node.as_span(),
            });
            return None;
        }
        Some(types::TraitMethod {
            name: as_field.name,
            def: as_field.def,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;
    use crate::types::{StructField, TraitMethod, Type, Variant};

    fn create_type_checker() -> TypeChecker {
        TypeChecker::new()
    }

    fn dummy_span() -> pest::Span<'static> {
        pest::Span::new("_", 0, 0).unwrap()
    }

    #[test]
    fn test_visit_type_declaration() {
        let mut checker = create_type_checker();
        let type_alias = ast::TypeAlias {
            name: "MyType".to_string(),
            params: None,
            definition: Box::new(ast::TypeDefinition::Type(ast::Type::Named(
                ast::NamedType {
                    name: "number".to_string(),
                    args: None,
                    span: dummy_span(),
                },
            ))),
            span: dummy_span(),
        };

        let result = checker.visit_type_declaration(&type_alias);
        assert_eq!(result, types::Type::Void);
        assert!(checker.errors.is_empty());

        let defined_type = checker.type_registry.lookup("MyType").unwrap();
        assert_eq!(defined_type, types::Type::Number);
    }

    #[test]
    fn test_visit_enum_definition() {
        let mut checker = create_type_checker();
        let enum_definition = ast::EnumDefinition {
            variants: vec![
                ast::VariantDefinition::Unit(ast::UnitVariant {
                    name: "Variant1".to_string(),
                    span: dummy_span(),
                }),
                ast::VariantDefinition::Struct(ast::StructVariant {
                    name: "Variant2".to_string(),
                    def: ast::StructDefinition {
                        fields: vec![ast::StructDefinitionField::Mandatory(
                            ast::StructMandatoryField {
                                name: "field".to_string(),
                                definition: ast::Type::Named(ast::NamedType {
                                    name: "number".to_string(),
                                    args: None,
                                    span: dummy_span(),
                                }),
                                span: dummy_span(),
                            },
                        )],
                        span: dummy_span(),
                    },
                    span: dummy_span(),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_enum_definition(&enum_definition);
        assert_eq!(
            result,
            types::EnumType {
                variants: vec![
                    Variant {
                        name: "Variant1".to_string(),
                        def: types::Type::Unit,
                    },
                    Variant {
                        name: "Variant2".to_string(),
                        def: types::Type::Struct(types::StructType {
                            fields: vec![types::StructField {
                                name: "field".to_string(),
                                def: types::Type::Number,
                                optional: false,
                            }],
                        }),
                    },
                ],
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_struct_definition() {
        let mut checker = create_type_checker();
        let struct_definition = ast::StructDefinition {
            fields: vec![
                ast::StructDefinitionField::Mandatory(ast::StructMandatoryField {
                    name: "field1".to_string(),
                    definition: ast::Type::Named(ast::NamedType {
                        name: "number".to_string(),
                        args: None,
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                }),
                ast::StructDefinitionField::Optional(ast::StructOptionalField {
                    name: "field2".to_string(),
                    default: ast::Expression::NumberLiteral(ast::NumberLiteral {
                        value: ordered_float::OrderedFloat(42.0),
                        span: dummy_span(),
                    }),
                    span: dummy_span(),
                }),
            ],
            span: dummy_span(),
        };

        let result = checker.visit_struct_definition(&struct_definition);
        assert_eq!(
            result,
            types::StructType {
                fields: vec![
                    StructField {
                        name: "field1".to_string(),
                        def: types::Type::Number,
                        optional: false,
                    },
                    StructField {
                        name: "field2".to_string(),
                        def: types::Type::Number,
                        optional: true,
                    },
                ],
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_trait_definition() {
        let mut checker = create_type_checker();
        let trait_definition = ast::TraitDefinition {
            name: "MyTrait".to_string(),
            body: Box::new(ast::StructDefinition {
                fields: vec![ast::StructDefinitionField::Mandatory(
                    ast::StructMandatoryField {
                        name: "method".to_string(),
                        definition: ast::Type::Function(ast::FunctionType {
                            params: vec![ast::Type::Named(ast::NamedType {
                                name: "number".to_string(),
                                args: None,
                                span: dummy_span(),
                            })],
                            returned: Box::new(ast::Type::Named(ast::NamedType {
                                name: "void".to_string(),
                                args: None,
                                span: dummy_span(),
                            })),
                            span: dummy_span(),
                        }),
                        span: dummy_span(),
                    },
                )],
                span: dummy_span(),
            }),
            span: dummy_span(),
        };

        let result = checker.visit_trait_definition(&trait_definition);
        assert_eq!(
            result,
            types::TraitType {
                methods: vec![TraitMethod {
                    name: "method".to_string(),
                    def: types::Type::Function(types::FunctionType {
                        params: vec![Type::Number],
                        return_type: Box::new(Type::Void),
                    }),
                }],
            }
        );
        assert!(checker.errors.is_empty());
    }

    #[test]
    fn test_visit_type_definition() {
        let mut checker = create_type_checker();
        let type_definition = ast::TypeDefinition::Type(ast::Type::Named(ast::NamedType {
            name: "string".to_string(),
            args: None,
            span: dummy_span(),
        }));

        let result = checker.visit_type_definition(&type_definition);
        assert_eq!(result, types::Type::String);
        assert!(checker.errors.is_empty());
    }
}
