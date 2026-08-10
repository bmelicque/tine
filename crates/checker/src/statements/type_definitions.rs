use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Locatable};
use tine_ir as ir;
use tine_macros::EnumFrom;
use tine_symbols::symbols::*;
use tine_types::{store::TypeStore, types};

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
            self.fallback_check_struct_body(body);
            return None;
        };
        if self.current_scope().has(name.as_str()) {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.text };
            self.error(error, name.loc);
            self.fallback_check_struct_body(body);
            return None;
        }
        let owner_id: StructSymbolId = self.insert(StructSymbol {
            name: name.text.clone(),
            defined_at: name.loc,
            ..Default::default()
        });

        let ((mut ty, members), params) = self.with_type_params(&node.params, |checker| {
            checker.visit_type_struct_body(body, owner_id.into(), false)
        });
        ty.set_params(params);

        let ty = self.intern_unique(ty);
        let owner = self.symbols.get_mut(owner_id);
        owner.ty = ty;
        owner.members = members;
        self.types.add_alias(ty, name.text);

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
            node.variants
                .into_iter()
                .map(|v| self.fallback_visit_variant_body(v.body))
                .for_each(drop);
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
            self.fallback_visit_variant_body(variant.body);
            return None;
        };
        let body = variant
            .body
            .map_or(vec![], |body| self.visit_variant_body(body, owner.into()));
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

    fn visit_type_struct_body(
        &mut self,
        body: ast::StructBody,
        owner: TypeSymbolId,
        is_enum: bool,
    ) -> (TypeBody, Vec<MemberSymbolId>) {
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
        (ty.into(), symbols)
    }

    fn visit_struct_definition_field(
        &mut self,
        owner: TypeSymbolId,
        field: ast::StructDefinitionField,
        is_enum: bool,
    ) -> Option<MemberSymbolId> {
        let ty = self.visit_type(field.definition?);
        let name = field.name?;
        Some(self.symbols.insert(MemberSymbol {
            name: name.text,
            public: field.public || is_enum,
            ty,
            owner,
            defined_at: name.loc,
            ..Default::default()
        }))
    }

    fn visit_variant_body(
        &mut self,
        body: ast::VariantBody,
        owner: TypeSymbolId,
    ) -> Vec<MemberSymbolId> {
        body.elements
            .into_iter()
            .enumerate()
            .map(|(i, (public, ty))| self.visit_variant_body_element(owner, ty, public, i))
            .collect::<Vec<_>>()
    }

    fn visit_variant_body_element(
        &mut self,
        owner: TypeSymbolId,
        ty: ast::Type,
        public: bool,
        i: usize,
    ) -> MemberSymbolId {
        let loc = ty.loc();
        let ty = self.visit_type(ty);
        self.symbols.insert(MemberSymbol {
            name: format!("_{}", i),
            public,
            ty,
            owner,
            defined_at: loc,
            ..Default::default()
        })
    }

    fn fallback_visit_variant_body(&mut self, body: Option<ast::VariantBody>) {
        if let Some(body) = body {
            body.elements
                .into_iter()
                .map(|e| self.visit_type(e.1))
                .for_each(drop);
        }
    }

    fn fallback_check_struct_body(&mut self, body: ast::StructBody) {
        body.fields
            .into_iter()
            .filter_map(|f| f.definition)
            .for_each(|def| {
                self.visit_type(def);
            })
    }
}
