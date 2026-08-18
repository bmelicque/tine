use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Locatable};
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
        if self.current_scope().has(name.as_str()) {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.text };
            self.error(error, name.loc);
            self.fallback_check_enum_body(node.items);
            return vec![];
        }
        let owner_id: EnumSymbolId = self.insert(EnumSymbol {
            name: name.text.clone(),
            defined_at: name.loc,
            ..Default::default()
        });
        let Some(items) = node.items else {
            let def = ir::EnumDefinition {
                loc: node.loc,
                symbol: owner_id,
            };
            return vec![def.into()];
        };
        let (mut self_, type_params) = self.with_type_params2(&node.params);

        let (variants, method_nodes) = split_enum_body(items);

        let variants = variants
            .into_iter()
            .filter_map(|v| self_.visit_enum_variant(v, owner_id))
            .collect::<Vec<_>>();
        let ty = self_.get_enum_type(&variants, type_params.to_vec());
        let mut self_ = self_.with_this(ty);
        self_.symbols.get_mut(owner_id).ty = ty;
        self_.types.add_alias(ty, name.text);
        variants.iter().for_each(|v| {
            self_.symbols.get_mut(*v).ty = ty;
        });

        let methods =
            self_.infer_method_symbols(&method_nodes, owner_id.into(), &SubstitutionTable::new());
        let symbol = self_.symbols.get_mut(owner_id);
        symbol.variants = variants.clone();
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

    fn get_enum_type(
        &mut self,
        variants: &[VariantSymbolId],
        params: Vec<types::TypeParam>,
    ) -> types::TypeId {
        let type_variants = variants
            .iter()
            .map(|v| types::Variant {
                name: self.symbol_name(*v).into(),
                def: self.symbol_type_id(*v),
            })
            .collect::<Vec<_>>();
        self.intern_unique(types::EnumType {
            params,
            variants: type_variants,
            ..Default::default()
        })
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
