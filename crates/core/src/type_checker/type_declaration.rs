use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, EnumType, GenericType, StructType, TupleType, Type, TypeId},
    SymbolData,
};

use super::TypeChecker;

impl TypeChecker {
    pub fn visit_type_declaration(&mut self, node: &ast::TypeAlias) -> TypeId {
        let params = match node.params {
            Some(ref params) => params,
            None => &vec![],
        };
        let (ty, params) = self.with_scope(node.definition.as_span(), |checker| {
            let mut param_types = Vec::new();
            for (i, param) in params.iter().enumerate() {
                let ty = types::TypeParam {
                    name: param.clone(),
                    idx: i,
                };
                let ty = checker.analysis_context.type_store.add(ty.into());
                param_types.push(ty);
                // FIXME: spans
                checker
                    .analysis_context
                    .register_symbol(SymbolData::new_type(param.clone(), ty, node.span));
            }
            (checker.visit_type_definition(&node.definition), param_types)
        });

        let name = &node.name;
        if self.analysis_context.find_in_current_scope(name).is_some() {
            self.error(format!("cannot redefine type '{}'", name), node.span);
            return TypeStore::VOID;
        }
        let ty = match params.len() {
            0 => ty,
            _ => self
                .analysis_context
                .type_store
                .add(Type::Generic(GenericType {
                    params,
                    definition: ty,
                })),
        };
        self.analysis_context
            .register_symbol(SymbolData::new_type(name.clone(), ty, node.span));
        self.analysis_context
            .type_store
            .add_alias(ty, name.to_string());

        TypeStore::VOID
    }

    fn visit_type_definition(&mut self, node: &ast::TypeDefinition) -> TypeId {
        match node {
            ast::TypeDefinition::Enum(e) => self.visit_enum_definition(e).into(),
            ast::TypeDefinition::Struct(s) => self.visit_struct_definition(s).into(),
            ast::TypeDefinition::Type(t) => self.visit_type(t),
        }
    }

    fn visit_enum_definition(&mut self, node: &ast::EnumDefinition) -> TypeId {
        let variants = node
            .variants
            .iter()
            .map(|variant| self.visit_variant_definition(variant))
            .collect();
        let id = self.analysis_context.type_store.get_next_id();
        self.analysis_context
            .type_store
            .add(Type::Enum(EnumType { id, variants }))
    }

    fn visit_variant_definition(&mut self, node: &ast::VariantDefinition) -> types::Variant {
        let def = match node {
            ast::VariantDefinition::Struct(s) => self.visit_struct_definition(&s.def),
            ast::VariantDefinition::Tuple(t) => {
                let elements = t.elements.iter().map(|el| self.visit_type(el)).collect();
                self.analysis_context
                    .type_store
                    .add(Type::Tuple(TupleType { elements }))
            }
            ast::VariantDefinition::Unit(_) => TypeStore::UNIT,
        };
        types::Variant {
            name: node.as_name(),
            def,
        }
    }

    fn visit_struct_definition(&mut self, node: &ast::StructDefinition) -> TypeId {
        let fields = node
            .fields
            .iter()
            .map(|field| self.visit_struct_definition_field(field))
            .collect();
        let id = self.analysis_context.type_store.get_next_id();
        self.analysis_context
            .type_store
            .add(Type::Struct(StructType { id, fields }))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast;

    fn create_type_checker() -> TypeChecker {
        TypeChecker::new(Vec::new())
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
            op: ast::DefinitionOp::Strict,
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
        assert_eq!(result, TypeStore::VOID);
        assert!(checker.errors.is_empty());

        let defined_type = checker.analysis_context.lookup("MyType").unwrap();
        let defined_type = checker.resolve(defined_type.borrow().ty).clone();
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
        let result = checker.resolve(result).clone();
        assert!(matches!(result, Type::Enum(_)));
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
        let result = checker.resolve(result).clone();
        assert!(matches!(result, Type::Struct(StructType { .. })));
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
        assert_eq!(result, TypeStore::STRING);
        assert!(checker.errors.is_empty());
    }
}
