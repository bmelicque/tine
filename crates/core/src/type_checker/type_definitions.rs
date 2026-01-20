use crate::{
    ast,
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, EnumType, GenericType, StructType, TupleType, Type, TypeId},
    Location, SymbolData, SymbolKind,
};

use super::TypeChecker;

impl TypeChecker<'_> {
    pub fn visit_type_alias(&mut self, node: &ast::TypeAlias) -> TypeId {
        let (ty, params) = self.visit_with_type_params(&node.params, node.loc, |checker| {
            checker.visit_type(&node.definition)
        });

        let ty = match params.len() {
            0 => ty,
            _ => self.intern(Type::Generic(GenericType {
                params,
                definition: ty,
            })),
        };

        self.add_type_to_scope(node.name.clone(), node.loc, ty);

        TypeStore::UNIT
    }

    pub fn visit_struct_definition(&mut self, node: &ast::StructDefinition) -> TypeId {
        let (ty, params) = self.visit_with_type_params(&node.params, node.loc, |checker| {
            checker.visit_type_body(&node.body)
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

    pub fn visit_enum_definition(&mut self, node: &ast::EnumDefinition) -> TypeId {
        let (ty, params) = self.visit_with_type_params(&node.params, node.loc, |checker| {
            let variants: Vec<types::Variant> = node
                .variants
                .iter()
                .map(|variant| checker.visit_enum_variant(variant))
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

    fn visit_enum_variant(&mut self, variant: &ast::VariantDefinition) -> types::Variant {
        let ty = match &variant.body {
            Some(body) => self.visit_type_body(body),
            None => TypeStore::UNIT,
        };
        types::Variant {
            name: variant.name.clone(),
            def: ty,
        }
    }

    fn visit_with_type_params<F>(
        &mut self,
        params: &Option<Vec<String>>,
        loc: Location,
        mut visit: F,
    ) -> (u32, Vec<u32>)
    where
        F: FnMut(&mut Self) -> TypeId,
    {
        let params = match params {
            Some(ref params) => params,
            None => &vec![],
        };
        self.with_scope(|checker| {
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
                    defined_at: loc,
                    ..Default::default()
                });
            }
            (visit(checker), param_types)
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
            .map(|field| self.visit_struct_definition_field(field))
            .collect();
        let id = 0;
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

    fn visit_tuple_body(&mut self, body: &ast::TupleType) -> TypeId {
        let elements = body.elements.iter().map(|el| self.visit_type(el)).collect();
        self.intern_unique(Type::Tuple(TupleType { elements }))
    }

    // OLD
    fn add_type_to_scope(&mut self, name: String, loc: Location, ty: TypeId) {
        if self.ctx.find_in_current_scope(&name).is_some() {
            self.error(format!("cannot redefine type '{}'", name), loc);
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
