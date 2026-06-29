use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{ast, ir, type_checker::TypeChecker, types, Location, SymbolData, SymbolKind};

impl TypeChecker<'_> {
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
                checker.ctx.register_symbol(SymbolData {
                    name: param.text.clone(),
                    ty: ty.id,
                    kind: SymbolKind::TypeAlias,
                    defined_at: param.loc,
                    ..Default::default()
                });
                param_types.push(ty);
            }
            (visit(checker), param_types)
        })
    }

    pub fn make_temp_variable(&mut self, at: Location, value: &ir::Expression) -> ir::Identifier {
        let ty = value.ty();
        self.make_temp_variable_with_type(at, value, ty)
    }
    pub fn make_temp_variable_with_type(
        &mut self,
        at: Location,
        value: &ir::Expression,
        ty: types::TypeId,
    ) -> ir::Identifier {
        let name = format!("${}", generate_id());
        let symbol = self.ctx.register_symbol(SymbolData {
            name,
            ty,
            kind: SymbolKind::constant(),
            defined_at: at,
            dependencies: value.dependencies().map(Into::into).collect::<Vec<_>>(),
            ..Default::default()
        });
        ir::Identifier { loc: at, symbol }
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn generate_id() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
