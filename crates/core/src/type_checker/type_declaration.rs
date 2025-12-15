use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, EnumType, GenericType, StructType, TupleType, Type, TypeId},
    SymbolData, SymbolKind,
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_type_declaration(&mut self, node: &ast::TypeAlias) -> TypeId {
        let params = match node.params {
            Some(ref params) => params,
            None => &vec![],
        };
        let (ty, params) = self.with_scope(|checker| {
            let mut param_types = Vec::new();
            for (i, param) in params.iter().enumerate() {
                let ty = types::TypeParam {
                    name: param.clone(),
                    idx: i,
                };
                let ty = checker.intern(ty.into());
                param_types.push(ty);
                // FIXME: spans
                checker.ctx.register_symbol(SymbolData {
                    name: param.clone(),
                    ty,
                    kind: SymbolKind::Type { members: vec![] },
                    defined_at: node.loc,
                    ..Default::default()
                });
            }
            (checker.visit_type_definition(&node.definition), param_types)
        });

        let name = &node.name;
        if self.ctx.find_in_current_scope(name).is_some() {
            self.error(format!("cannot redefine type '{}'", name), node.loc);
            return TypeStore::UNIT;
        }
        let ty = match params.len() {
            0 => ty,
            _ => self.intern_unique(Type::Generic(GenericType {
                params,
                definition: ty,
            })),
        };
        self.ctx.register_symbol(SymbolData {
            name: name.clone(),
            ty,
            kind: SymbolKind::Type { members: vec![] },
            defined_at: node.loc,
            ..Default::default()
        });
        self.ctx.type_store.add_alias(ty, name.to_string());

        TypeStore::UNIT
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
        let id = self.ctx.type_store.get_next_id();
        self.intern_unique(Type::Enum(EnumType { id, variants }))
    }

    fn visit_variant_definition(&mut self, node: &ast::VariantDefinition) -> types::Variant {
        let def = match node {
            ast::VariantDefinition::Struct(s) => self.visit_struct_definition(&s.def),
            ast::VariantDefinition::Tuple(t) => {
                let elements = t.elements.iter().map(|el| self.visit_type(el)).collect();
                self.intern(Type::Tuple(TupleType { elements }))
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
        let id = self.ctx.type_store.get_next_id();
        self.intern_unique(Type::Struct(StructType { id, fields }))
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
    use crate::{analyzer::session::Session, ast, Location};

    fn create_type_checker() -> TypeChecker<'static> {
        let session = Box::leak(Box::new(Session::new()));
        TypeChecker::new(session, 0)
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
                    loc: Location::dummy(),
                },
            ))),
            loc: Location::dummy(),
        };

        let result = checker.visit_type_declaration(&type_alias);
        assert_eq!(result, TypeStore::UNIT);
        assert!(checker.errors.is_empty());

        let defined_type = checker.ctx.lookup("MyType").unwrap();
        let defined_type = checker.resolve(defined_type.borrow().get_type()).clone();
        assert_eq!(defined_type, types::Type::Number);
    }

    #[test]
    fn test_visit_enum_definition() {
        let mut checker = create_type_checker();
        let enum_definition = ast::EnumDefinition {
            variants: vec![
                ast::VariantDefinition::Unit(ast::UnitVariant {
                    name: "Variant1".to_string(),
                    loc: Location::dummy(),
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
                                    loc: Location::dummy(),
                                }),
                                loc: Location::dummy(),
                            },
                        )],
                        loc: Location::dummy(),
                    },
                    loc: Location::dummy(),
                }),
            ],
            loc: Location::dummy(),
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
                        loc: Location::dummy(),
                    }),
                    loc: Location::dummy(),
                }),
                ast::StructDefinitionField::Optional(ast::StructOptionalField {
                    name: "field2".to_string(),
                    default: ast::Expression::NumberLiteral(ast::NumberLiteral {
                        value: ordered_float::OrderedFloat(42.0),
                        loc: Location::dummy(),
                    }),
                    loc: Location::dummy(),
                }),
            ],
            loc: Location::dummy(),
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
            loc: Location::dummy(),
        }));

        let result = checker.visit_type_definition(&type_definition);
        assert_eq!(result, TypeStore::STRING);
        assert!(checker.errors.is_empty());
    }
}
