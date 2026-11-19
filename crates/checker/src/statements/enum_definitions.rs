use tine_ast as ast;
use tine_common::locations::Locatable;
use tine_ir as ir;
use tine_symbols::symbols::*;
use tine_types::types;

use crate::{substitutions::SubstitutionTable, TypeChecker};

impl TypeChecker {
    pub fn visit_enum_definition(&mut self, node: ast::EnumDefinition) -> Vec<ir::Statement> {
        let Some(name) = node.name else {
            self.fallback_check_enum_body(node.items);
            return vec![];
        };
        if self.check_duplicate_identifier(&name) {
            self.fallback_check_enum_body(node.items);
            return vec![];
        }
        let owner_id: EnumSymbolId = self.insert(EnumSymbol {
            public: node.public,
            docs: node.docs.map(|d| d.text),
            name: name.text.clone(),
            defined_at: name.loc,
            ..Default::default()
        });
        let Some(items) = node.items else {
            let loc = node.loc;
            let symbol = owner_id;
            let def = ir::EnumDefinition { loc, symbol };
            return vec![def.into()];
        };

        let (variants, method_nodes) = split_enum_body(items);
        let (mut self_, type_params) = self.with_type_params2(&node.params);

        // Variants
        let this = self_.declare_temporary_type(&name, &type_params);
        let variants = variants
            .into_iter()
            .filter_map(|v| self_.visit_enum_variant(v, owner_id))
            .collect::<Vec<_>>();
        let ty = self_.attach_variant_symbols(owner_id, variants, type_params.to_vec(), this);

        let mut self_ = self_.with_this(ty);
        let methods =
            self_.infer_method_symbols(&method_nodes, owner_id.into(), &SubstitutionTable::new());
        let symbol = self_.symbols.get_mut(owner_id);
        symbol.methods.extend(methods.values().copied());

        let def = ir::EnumDefinition {
            loc: node.loc,
            symbol: owner_id.into(),
        };
        let mut stmts: Vec<ir::Statement> = self_
            .visit_methods(&method_nodes, methods)
            .into_iter()
            .map(Into::into)
            .collect();
        stmts.insert(0, def.into());
        stmts
    }

    fn visit_enum_variant(
        &mut self,
        variant: ast::VariantDefinition,
        owner: EnumSymbolId,
    ) -> Option<VariantSymbolId> {
        let Some(ident) = variant.name else {
            self.fallback_check_variant_body(variant.body);
            return None;
        };
        let body = variant
            .body
            .map_or(vec![], |body| self.visit_variant_body(body, owner.into()));

        Some(self.symbols.insert(VariantSymbol {
            docs: variant.docs.map(|d| d.text),
            name: ident.text,
            owner,
            body,
            defined_at: variant.loc,
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

    pub fn attach_variant_symbols(
        &mut self,
        e: EnumSymbolId,
        variants: Vec<VariantSymbolId>,
        type_params: Vec<types::TypeParam>,
        placeholder: types::TypeId,
    ) -> types::TypeId {
        let ty = self.get_enum_type(&variants, type_params, placeholder);
        let name = self.symbol_name(e).to_string();
        self.types.add_alias(ty, name);
        variants.iter().for_each(|v| {
            self.symbols.get_mut(*v).ty = ty;
        });
        let symbol = self.symbols.get_mut(e);
        symbol.ty = ty;
        symbol.variants = variants;
        ty
    }

    pub fn get_enum_type(
        &mut self,
        variants: &[VariantSymbolId],
        params: Vec<types::TypeParam>,
        placeholder: types::TypeId,
    ) -> types::TypeId {
        let type_variants = variants
            .iter()
            .map(|v| types::Variant {
                name: self.symbol_name(*v).into(),
                def: self.symbol_type_id(*v),
            })
            .collect::<Vec<_>>();
        self.types.overwrite(
            placeholder,
            types::EnumType {
                params,
                variants: type_variants,
                ..Default::default()
            },
        );
        placeholder
    }

    fn fallback_check_enum_body(&mut self, body: Option<Vec<ast::EnumItem>>) {
        let Some(body) = body else { return };
        body.into_iter().for_each(|item| {
            self.fallback_check_enum_item(item);
        })
    }

    fn fallback_check_enum_item(&mut self, item: ast::EnumItem) {
        match item {
            ast::EnumItem::Variant(v) => {
                self.fallback_check_variant_body(v.body);
            }
            ast::EnumItem::Method(_) => {}
        }
    }

    fn fallback_check_variant_body(&mut self, body: Option<ast::VariantBody>) {
        if let Some(body) = body {
            body.elements
                .into_iter()
                .map(|e| self.visit_type(e.1))
                .for_each(drop);
        }
    }
}

fn split_enum_body(
    body: Vec<ast::EnumItem>,
) -> (Vec<ast::VariantDefinition>, Vec<ast::MethodDefinition>) {
    let mut variants = Vec::new();
    let mut methods = Vec::new();

    for item in body {
        match item {
            ast::EnumItem::Variant(i) => variants.push(i),
            ast::EnumItem::Method(i) => methods.push(i),
        }
    }

    (variants, methods)
}
