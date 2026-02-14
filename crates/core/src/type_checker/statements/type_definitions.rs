use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, EnumType, GenericType, StructType, TupleType, Type, TypeId},
    DiagnosticKind, Location, SymbolData, SymbolKind,
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_type_alias(&mut self, node: &ast::TypeAlias) -> TypeId {
        let (ty, params) = if let Some(definition) = &node.definition {
            self.with_type_params(&node.params, |checker| checker.visit_type(definition))
        } else {
            (TypeStore::UNKNOWN, vec![])
        };

        let ty = match params.len() {
            0 => ty,
            _ => self.intern(Type::Generic(GenericType {
                params,
                definition: ty,
            })),
        };

        if let Some(ref name) = node.name {
            self.add_type_to_scope(name.text.clone(), node.loc, ty);
        }

        TypeStore::UNIT
    }

    pub fn visit_struct_definition(&mut self, node: &ast::StructDefinition) -> TypeId {
        let (ty, params) = match &node.body {
            Some(body) => {
                self.with_type_params(&node.params, |checker| checker.visit_type_body(body))
            }
            None => (TypeStore::UNKNOWN, vec![]),
        };

        let ty = match params.len() {
            0 => ty,
            _ => self.intern_unique(Type::Generic(GenericType {
                params,
                definition: ty,
            })),
        };

        if let Some(name) = &node.name {
            self.add_type_to_scope(name.text.clone(), node.loc, ty);
        }

        TypeStore::UNIT
    }

    pub fn visit_enum_definition(&mut self, node: &ast::EnumDefinition) -> TypeId {
        let (ty, params) = self.with_type_params(&node.params, |checker| {
            let variants: Vec<types::Variant> = node
                .variants
                .iter()
                .filter_map(|variant| checker.visit_enum_variant(variant))
                .collect();

            checker.intern_unique(Type::Enum(EnumType { id: 0, variants }))
        });

        let ty = match params.len() {
            0 => ty,
            _ => self.intern_unique(Type::Generic(GenericType {
                params,
                definition: ty,
            })),
        };

        self.add_type_to_scope(node.name.clone(), node.loc, ty);

        TypeStore::UNIT
    }

    fn visit_enum_variant(&mut self, variant: &ast::VariantDefinition) -> Option<types::Variant> {
        let ty = match &variant.body {
            Some(body) => self.visit_type_body(body),
            None => TypeStore::UNIT,
        };

        variant.name.as_ref().map(|name| types::Variant {
            name: name.text.clone(),
            def: ty,
        })
    }

    fn visit_type_body(&mut self, body: &ast::TypeBody) -> TypeId {
        match body {
            ast::TypeBody::Struct(body) => self.visit_struct_def_body(body),
            ast::TypeBody::Tuple(body) => self.visit_tuple_body(body),
        }
    }

    fn visit_struct_def_body(&mut self, body: &ast::StructBody) -> TypeId {
        let fields = body
            .fields
            .iter()
            .filter_map(|field| self.visit_struct_definition_field(field))
            .collect();
        let id = 0;
        self.intern_unique(Type::Struct(StructType { id, fields }))
    }

    fn visit_struct_definition_field(
        &mut self,
        field: &ast::StructDefinitionField,
    ) -> Option<types::StructField> {
        let name = field.as_name();
        let def = match field {
            ast::StructDefinitionField::Mandatory(ref field) => match &field.definition {
                Some(def) => self.visit_type(def),
                None => TypeStore::UNKNOWN,
            },
            ast::StructDefinitionField::Optional(field) => match &field.default {
                Some(def) => self.visit_expression(def),
                None => TypeStore::UNKNOWN,
            },
        };
        match name {
            Some(name) => Some(types::StructField {
                name: name.text.clone(),
                def,
                optional: field.is_optional(),
            }),
            None => None,
        }
    }

    fn visit_tuple_body(&mut self, body: &ast::TupleType) -> TypeId {
        let elements = body.elements.iter().map(|el| self.visit_type(el)).collect();
        self.intern_unique(Type::Tuple(TupleType { elements }))
    }

    // OLD
    fn add_type_to_scope(&mut self, name: String, loc: Location, ty: TypeId) {
        if self.ctx.find_in_current_scope(&name).is_some() {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.clone() };
            self.error(error, loc);
            return;
        }

        self.ctx.register_symbol(SymbolData {
            name: name.clone(),
            ty,
            kind: SymbolKind::Type { members: vec![] },
            defined_at: loc,
            ..Default::default()
        });
        self.session.types().add_alias(ty, name.to_string());
    }
}
