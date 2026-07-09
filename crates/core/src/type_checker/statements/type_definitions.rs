use enum_from_derive::EnumFrom;

use crate::{
    ast, ir,
    type_checker::{symbols::*, type_store::TypeStore},
    types, DiagnosticKind,
};

use super::TypeChecker;

#[derive(Debug, EnumFrom)]
pub enum TypeBody {
    Struct(types::StructType),
    Tuple(types::TupleType),
}
impl TypeBody {
    fn set_params(&mut self, type_params: Vec<types::TypeParam>) {
        match self {
            Self::Struct(ty) => ty.params = type_params,
            Self::Tuple(ty) => ty.params = type_params,
        }
    }
}
impl Into<types::Type> for TypeBody {
    fn into(self) -> types::Type {
        match self {
            Self::Struct(ty) => ty.into(),
            Self::Tuple(ty) => ty.into(),
        }
    }
}

impl TypeChecker {
    pub fn visit_type_alias(&mut self, node: ast::TypeAlias) {
        let (ty, params) = if let Some(definition) = node.definition {
            self.with_type_params(&node.params, |checker| checker.visit_type(definition))
        } else {
            (TypeStore::UNKNOWN, vec![])
        };

        let ty = match params.len() {
            0 => ty,
            _ => self.intern(types::GenericDef { params, def: ty }),
        };

        if let Some(name) = node.name {
            self.symbols.insert::<TypeAliasSymbolId>(TypeAliasSymbol {
                name: name.text.clone(),
                ty,
                defined_at: node.loc,
                ..Default::default()
            });
            self.types.add_alias(ty, name.text);
        }
    }

    pub fn visit_struct_definition(
        &mut self,
        node: ast::StructDefinition,
    ) -> Option<ir::StructDefinition> {
        let Some(body) = node.body else { return None };
        let Some(name) = node.name else {
            self.fallback_check_body(body);
            return None;
        };
        if self.current_scope().has(name.as_str()) {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.text };
            self.error(error, name.loc);
            self.fallback_check_body(body);
            return None;
        }
        let owner_id: StructSymbolId = self.insert(StructSymbol {
            name: name.text.clone(),
            defined_at: node.loc,
            ..Default::default()
        });

        let ((mut ty, body), params) = self.with_type_params(&node.params, |checker| {
            checker.visit_type_body(body, owner_id.into(), false)
        });
        ty.set_params(params);

        let ty = self.intern_unique(ty);
        let owner = self.symbols.get_mut(owner_id);
        owner.ty = ty;
        owner.body = body;

        Some(ir::StructDefinition {
            loc: node.loc,
            symbol: owner_id.into(),
        })
    }

    pub fn visit_enum_definition(
        &mut self,
        node: ast::EnumDefinition,
    ) -> Option<ir::EnumDefinition> {
        let name = node.name?;
        if self.current_scope().has(name.as_str()) {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.text };
            self.error(error, name.loc);
            self.fallback_check_variants(node.variants);
            return None;
        }
        let owner_id: EnumSymbolId = self.insert(EnumSymbol {
            name: name.text.clone(),
            defined_at: node.loc,
            ..Default::default()
        });

        let (variants, params) = self.with_type_params(&node.params, |self_| {
            node.variants
                .into_iter()
                .filter_map(|variant| self_.visit_enum_variant(variant, owner_id))
                .collect::<Vec<_>>()
        });
        let type_variants = variants
            .iter()
            .map(|v| types::Variant {
                name: self.symbol_name(*v).into(),
                def: self.symbol_type_id(*v),
            })
            .collect::<Vec<_>>();
        let ty = self.intern_unique(types::EnumType {
            id: 0,
            params,
            variants: type_variants,
        });

        let owner = self.symbols.get_mut(owner_id);
        owner.ty = ty;
        owner.variants = variants;

        Some(ir::EnumDefinition {
            loc: node.loc,
            symbol: owner_id,
        })
    }

    fn visit_enum_variant(
        &mut self,
        variant: ast::VariantDefinition,
        owner: EnumSymbolId,
    ) -> Option<VariantSymbolId> {
        let Some(ident) = variant.name else {
            if let Some(body) = variant.body {
                self.fallback_check_body(body);
            }
            return None;
        };
        let body = variant
            .body
            .map(|body| self.visit_type_body(body, owner.into(), true).1);
        let ty = self.symbol_type_id(owner);

        Some(self.symbols.insert(VariantSymbol {
            name: ident.text,
            ty,
            owner,
            body,
            defined_at: variant.loc,
            ..Default::default()
        }))
    }

    /// return the final type with the symbol body
    fn visit_type_body(
        &mut self,
        body: ast::TypeBody,
        owner: TypeSymbolId,
        is_enum: bool,
    ) -> (TypeBody, TypeSymbolBody) {
        match body {
            ast::TypeBody::Struct(body) => self.visit_type_struct_body(body, owner, is_enum),
            ast::TypeBody::Tuple(body) => self.visit_type_tuple_body(body, owner, is_enum),
        }
    }

    fn visit_type_struct_body(
        &mut self,
        body: ast::StructBody,
        owner: TypeSymbolId,
        is_enum: bool,
    ) -> (TypeBody, TypeSymbolBody) {
        let symbols = body
            .fields
            .into_iter()
            .filter_map(|field| self.visit_struct_definition_field(owner, field, is_enum))
            .collect::<Vec<_>>();
        let fields = symbols
            .iter()
            .map(|s| types::StructField {
                name: self.symbol_name(*s).into(),
                def: self.symbol_type_id(*s),
            })
            .collect();
        let id = 0;
        let ty = types::StructType {
            id,
            fields,
            ..Default::default()
        };
        let body = TypeSymbolBody::Struct(
            symbols
                .into_iter()
                .map(|s| (self.symbol_name(s).to_string(), s))
                .collect(),
        );
        (ty.into(), body)
    }

    fn visit_struct_definition_field(
        &mut self,
        owner: TypeSymbolId,
        field: ast::StructDefinitionField,
        is_enum: bool,
    ) -> Option<MemberSymbolId> {
        let ty = self.visit_type(field.definition?);
        Some(self.symbols.insert(MemberSymbol {
            name: field.name?.text,
            public: field.public || is_enum,
            ty,
            owner,
            defined_at: field.loc,
            ..Default::default()
        }))
    }

    fn visit_type_tuple_body(
        &mut self,
        body: ast::TupleBody,
        owner: TypeSymbolId,
        is_enum: bool,
    ) -> (TypeBody, TypeSymbolBody) {
        let symbols = body
            .elements
            .into_iter()
            .enumerate()
            .map(|(i, (public, ty))| {
                let loc = ty.loc();
                let ty = self.visit_type(ty);
                self.symbols.insert(MemberSymbol {
                    name: format!("_{}", i),
                    public: public || is_enum,
                    ty,
                    owner,
                    defined_at: loc,
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let elements = symbols.iter().map(|s| self.symbol_type_id(*s)).collect();
        let ty = types::TupleType {
            elements,
            ..Default::default()
        };
        let body = TypeSymbolBody::Tuple(symbols);
        (ty.into(), body)
    }

    fn fallback_check_body(&mut self, body: ast::TypeBody) {
        match body {
            ast::TypeBody::Struct(s) => {
                s.fields
                    .into_iter()
                    .filter_map(|f| f.definition)
                    .for_each(|def| {
                        self.visit_type(def);
                    })
            }
            ast::TypeBody::Tuple(t) => {
                t.elements.into_iter().for_each(|(_, ty)| {
                    self.visit_type(ty);
                });
            }
        }
    }

    fn fallback_check_variants(&mut self, body: Vec<ast::VariantDefinition>) {
        body.into_iter()
            .filter_map(|v| v.body)
            .for_each(|b| self.fallback_check_body(b));
    }
}
