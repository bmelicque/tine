use enum_from_derive::EnumFrom;

use crate::{
    ast, ir,
    type_checker::{
        analysis_context::{symbols::TypeSymbolBody, type_store::TypeStore},
        SymbolHandle,
    },
    types, DiagnosticKind, Location, SymbolData, SymbolKind, SymbolRef,
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

impl TypeChecker<'_> {
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

        if let Some(ref name) = node.name {
            self.add_type_to_scope(name.text.clone(), node.loc, ty, SymbolKind::TypeAlias);
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
        let owner = self.add_type_to_scope(
            name.text.clone(),
            node.loc,
            TypeStore::UNKNOWN,
            SymbolKind::Struct {
                body: TypeSymbolBody::Struct(vec![]),
                methods: vec![],
            },
        );
        let Some(owner) = owner else {
            self.fallback_check_body(body);
            return None;
        };

        let ((mut ty, body), params) = self.with_type_params(&node.params, |checker| {
            checker.visit_type_body(body, owner.readonly())
        });
        ty.set_params(params);

        owner.borrow().ty = self.intern_unique(ty);
        owner.borrow().kind = SymbolKind::Struct {
            body,
            methods: vec![],
        };

        Some(ir::StructDefinition {
            loc: node.loc,
            name: ir::Identifier {
                loc: name.loc,
                symbol: owner.readonly(),
            },
        })
    }

    pub fn visit_enum_definition(
        &mut self,
        node: ast::EnumDefinition,
    ) -> Option<ir::EnumDefinition> {
        let name = node.name?;
        let owner = self.add_type_to_scope(
            name.as_str().to_string(),
            node.loc,
            TypeStore::UNKNOWN,
            SymbolKind::Enum {
                variants: vec![],
                methods: vec![],
            },
        );
        let Some(owner) = owner else {
            self.fallback_check_variants(node.variants);
            return None;
        };

        let (variants, params) = self.with_type_params(&node.params, |self_| {
            node.variants
                .into_iter()
                .filter_map(|variant| self_.visit_enum_variant(variant, owner.readonly()))
                .collect::<Vec<_>>()
        });
        let type_variants = variants
            .iter()
            .map(|v| types::Variant {
                name: v.as_name(),
                def: v.as_type(),
            })
            .collect::<Vec<_>>();
        let ty = self.intern_unique(types::EnumType {
            id: 0,
            params,
            variants: type_variants,
        });

        owner.borrow().ty = ty;
        owner.borrow().kind = SymbolKind::Enum {
            variants,
            methods: vec![],
        };

        Some(ir::EnumDefinition {
            loc: node.loc,
            name: ir::Identifier {
                loc: name.loc,
                symbol: owner.readonly(),
            },
        })
    }

    fn visit_enum_variant(
        &mut self,
        variant: ast::VariantDefinition,
        owner: SymbolRef,
    ) -> Option<SymbolRef> {
        let Some(ident) = variant.name else {
            if let Some(body) = variant.body {
                self.fallback_check_body(body);
            }
            return None;
        };

        let body = variant
            .body
            .map(|body| self.visit_type_body(body, owner.clone()).1);

        Some(self.ctx.register_symbol(SymbolData {
            name: ident.text,
            ty: owner.as_type(),
            kind: SymbolKind::Constructor { owner, body },
            defined_at: variant.loc,
            ..Default::default()
        }))
    }

    /// return the final type with the symbol body
    fn visit_type_body(
        &mut self,
        body: ast::TypeBody,
        owner: SymbolRef,
    ) -> (TypeBody, TypeSymbolBody) {
        match body {
            ast::TypeBody::Struct(body) => self.visit_type_struct_body(body, owner),
            ast::TypeBody::Tuple(body) => self.visit_type_tuple_body(body, owner),
        }
    }

    fn visit_type_struct_body(
        &mut self,
        body: ast::StructBody,
        owner: SymbolRef,
    ) -> (TypeBody, TypeSymbolBody) {
        let symbols = body
            .fields
            .into_iter()
            .filter_map(|field| self.visit_struct_definition_field(owner.clone(), field))
            .collect::<Vec<_>>();
        let fields = symbols
            .iter()
            .map(|s| types::StructField::from(s))
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
                .map(|s| (s.borrow().name.clone(), s.clone()))
                .collect(),
        );
        (ty.into(), body)
    }

    fn visit_struct_definition_field(
        &mut self,
        owner: SymbolRef,
        field: ast::StructDefinitionField,
    ) -> Option<SymbolRef> {
        let ty = self.visit_type(field.definition?);
        Some(self.ctx.register_symbol(SymbolData {
            name: field.name?.text,
            ty,
            kind: SymbolKind::Member { owner },
            defined_at: field.loc,
            ..Default::default()
        }))
    }

    fn visit_type_tuple_body(
        &mut self,
        body: ast::TupleType,
        owner: SymbolRef,
    ) -> (TypeBody, TypeSymbolBody) {
        let symbols = body
            .elements
            .into_iter()
            .enumerate()
            .map(|(i, ty)| {
                let loc = ty.loc();
                let ty = self.visit_type(ty);
                self.ctx.register_symbol(SymbolData {
                    name: format!("_{}", i),
                    ty,
                    kind: SymbolKind::Member {
                        owner: owner.clone(),
                    },
                    defined_at: loc,
                    ..Default::default()
                })
            })
            .collect::<Vec<_>>();
        let elements = symbols.iter().map(|s| s.borrow().ty).collect();
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
                self.visit_tuple_type(t);
            }
        }
    }

    fn fallback_check_variants(&mut self, body: Vec<ast::VariantDefinition>) {
        body.into_iter()
            .filter_map(|v| v.body)
            .for_each(|b| self.fallback_check_body(b));
    }

    // OLD
    fn add_type_to_scope(
        &mut self,
        name: String,
        loc: Location,
        ty: types::TypeId,
        kind: SymbolKind,
    ) -> Option<SymbolHandle> {
        if self.ctx.find_in_current_scope(&name).is_some() {
            let error = DiagnosticKind::DuplicateIdentifier { name };
            self.error(error, loc);
            return None;
        }

        let symbol = self.ctx.register_symbol(SymbolData {
            name: name.clone(),
            ty,
            kind,
            defined_at: loc,
            ..Default::default()
        });
        self.session.types().add_alias(ty, name);
        self.session.get_handle(symbol)
    }
}
