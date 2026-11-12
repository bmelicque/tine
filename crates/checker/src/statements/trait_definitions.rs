use tine_ast as ast;
use tine_common::diagnostics::DiagnosticKind;
use tine_symbols::symbols::{self, *};
use tine_types::types;

use crate::{substitutions::SubstitutionTable, TypeChecker};

impl TypeChecker {
    pub(crate) fn visit_trait_definition(&mut self, node: ast::TraitDefinition) {
        let Some(name) = node.name else {
            return;
        };
        if self.current_scope().has(name.as_str()) {
            let error = DiagnosticKind::DuplicateIdentifier { name: name.text };
            self.error(error, name.loc);
            return;
        }

        let symbol_id = self.insert::<TraitSymbolId>(TraitSymbol {
            name: name.text.clone(),
            public: node.public,
            docs: node.docs.map(|d| d.text),
            defined_at: name.loc,
            ..Default::default()
        });

        let (methods, type_params) =
            self.visit_trait_methods(symbol_id.into(), &node.params, node.methods);

        let trait_methods = methods
            .iter()
            .map(|m| self.get_method_signature(*m))
            .collect();
        let type_param_names = type_params.iter().map(|p| p.name.clone()).collect();

        let ty = self.intern(types::TraitType {
            params: type_params,
            methods: trait_methods,
        });

        let symbol = self.symbols.get_mut(symbol_id);
        symbol.param_names = type_param_names;
        symbol.ty = ty;
        symbol.methods = methods;
        self.types.add_alias(ty, name.text);
    }

    fn visit_trait_methods(
        &mut self,
        owner: symbols::TypeSymbolId,
        type_params: &Option<Vec<ast::Identifier>>,
        methods: Option<Vec<ast::TraitMethod>>,
    ) -> (Vec<symbols::MethodSymbolId>, Vec<types::TypeParam>) {
        let (mut self_, type_params) = self.with_type_params2(type_params);
        let methods = methods
            .unwrap_or(vec![])
            .into_iter()
            .filter_map(|m| self_.infer_method_symbol(&m, owner, &SubstitutionTable::new()))
            .collect::<Vec<_>>();
        (methods, type_params)
    }

    fn get_method_signature(&self, symbol: symbols::MethodSymbolId) -> types::TraitMethod {
        let symbol = self.symbols.get(symbol);
        types::TraitMethod {
            self_type: None,
            name: symbol.name.clone(),
            def: symbol.ty,
        }
    }
}
