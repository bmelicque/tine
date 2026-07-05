use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    ast, ir,
    type_checker::{symbols::*, TypeChecker},
    types, Location,
};

impl TypeChecker {
    /// Create a local scope containing the given type params, then run the
    /// `visit` function inside that scope.
    pub fn with_type_params<F, R>(
        &mut self,
        params: &Option<Vec<ast::Identifier>>,
        visit: F,
    ) -> (R, Vec<types::TypeParam>)
    where
        F: FnOnce(&mut Self) -> R,
    {
        let params = match &params {
            Some(params) => params,
            None => &vec![],
        };
        self.with_scope(|checker| {
            let mut param_types = Vec::new();
            for param in params {
                let ty = checker.add_type_param(param.text.clone());
                // FIXME: spans
                checker
                    .symbols
                    .insert::<TypeAliasSymbolId>(TypeAliasSymbol {
                        name: param.text.clone(),
                        ty: ty.id,
                        defined_at: param.loc,
                        ..Default::default()
                    });
                checker.types.add_alias(ty.id, param.text.clone());
                param_types.push(ty);
            }
            (visit(checker), param_types)
        })
    }

    pub fn make_temp_variable(
        &mut self,
        at: Location,
        value: &ir::Expression,
    ) -> (ir::Identifier, VariableSymbolId) {
        let ty = value.ty();
        let symbol = self.make_temp_variable_with_type(at, value, ty);
        let node = ir::Identifier {
            loc: at,
            symbol: symbol.into(),
            ty,
        };
        (node, symbol)
    }
    pub fn make_temp_variable_with_type(
        &mut self,
        at: Location,
        value: &ir::Expression,
        ty: types::TypeId,
    ) -> VariableSymbolId {
        let name = format!("${}", generate_id());
        let dependencies = self
            .dependencies(value)
            .filter_map(|i| i.symbol.as_variable())
            .collect();
        self.symbols.insert::<VariableSymbolId>(VariableSymbol {
            name,
            ty,
            defined_at: at,
            dependencies,
            ..Default::default()
        })
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn generate_id() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
