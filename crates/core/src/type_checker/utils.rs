use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{
    ast,
    type_checker::{patterns::Binding, TypeChecker},
    types, Location, SymbolData, SymbolKind,
};

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
}

pub fn make_simple_declaration(binding: Binding) -> ast::VariableDeclaration {
    ast::VariableDeclaration {
        loc: binding.id.loc,
        mutable: binding.mutable,
        pattern: Some(ast::Pattern::Identifier(binding.id.into())),
        value: Some(binding.value),
        ..Default::default()
    }
}

static COUNTER: AtomicUsize = AtomicUsize::new(0);

fn generate_id() -> usize {
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

pub fn make_tmp_identifier(at: Location) -> ast::Identifier {
    ast::Identifier {
        loc: at,
        text: format!("${}", generate_id()),
    }
}

pub fn make_tmp_declaration(value: ast::Expression) -> ast::VariableDeclaration {
    ast::VariableDeclaration {
        loc: value.loc(),
        pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern(
            make_tmp_identifier(value.loc()),
        ))),
        value: Some(value),
        ..Default::default()
    }
}
