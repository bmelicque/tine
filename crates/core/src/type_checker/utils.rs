use std::sync::atomic::{AtomicUsize, Ordering};

use crate::{ast, type_checker::TypeChecker, types, Location, SymbolData, SymbolKind};

impl TypeChecker<'_> {
    pub fn with_type_params<F, R>(
        &mut self,
        params: &Option<Vec<ast::Identifier>>,
        visit: F,
    ) -> (R, Vec<types::TypeId>)
    where
        F: FnOnce(&mut Self) -> R,
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
                let ty = checker.intern(ty);
                param_types.push(ty);
                // FIXME: spans
                checker.ctx.register_symbol(SymbolData {
                    name: param.text.clone(),
                    ty,
                    kind: SymbolKind::TypeAlias,
                    defined_at: param.loc,
                    ..Default::default()
                });
            }
            (visit(checker), param_types)
        })
    }
}

pub fn make_simple_declaration(
    identifier: ast::Identifier,
    value: ast::Expression,
) -> ast::VariableDeclaration {
    ast::VariableDeclaration {
        docs: None,
        loc: identifier.loc,
        keyword: ast::DeclarationKeyword::Const,
        pattern: Some(ast::Pattern::Identifier(identifier.into())),
        value: Some(value),
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
        docs: None,
        loc: value.loc(),
        keyword: ast::DeclarationKeyword::Const,
        pattern: Some(ast::Pattern::Identifier(ast::IdentifierPattern(
            make_tmp_identifier(value.loc()),
        ))),
        value: Some(value),
    }
}
