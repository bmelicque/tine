use crate::{
    ast,
    type_checker::{analysis_context::symbols::TypeSymbolKind, TypeChecker},
    types, SymbolData, SymbolKind,
};

impl TypeChecker<'_> {
    pub fn with_type_params<F, R>(
        &mut self,
        params: &Option<Vec<ast::Identifier>>,
        mut visit: F,
    ) -> (R, Vec<u32>)
    where
        F: FnMut(&mut Self) -> R,
    {
        let params = match params {
            Some(ref params) => params,
            None => &vec![],
        };
        self.with_scope(|checker| {
            let mut param_types = Vec::new();
            for (i, param) in params.iter().enumerate() {
                let ty = types::TypeParam {
                    name: param.text.clone(),
                    idx: i,
                };
                let ty = checker.intern(ty.into());
                param_types.push(ty);
                // FIXME: spans
                checker.ctx.register_symbol(SymbolData {
                    name: param.text.clone(),
                    ty,
                    kind: SymbolKind::Type {
                        kind: TypeSymbolKind::Alias,
                        members: vec![],
                    },
                    defined_at: param.loc,
                    ..Default::default()
                });
            }
            (visit(checker), param_types)
        })
    }
}
