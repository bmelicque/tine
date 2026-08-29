use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir::{self as ir, Typed};
use tine_symbols::symbols::*;
use tine_types::types;

use crate::{
    substitutions::{SubstitutionTable, Substitutions},
    TypeChecker,
};

impl TypeChecker {
    pub fn find_method<F>(
        &mut self,
        host: types::TypeId,
        name: &str,
        mut extra: F,
    ) -> Option<(MethodSymbolId, types::TypeId)>
    where
        F: FnMut(&MethodSymbol) -> bool,
    {
        let host = self.deref_type(host);
        let (host, sub) = self.unwrap_type(host);
        let type_symbol = self.get_type_symbol_id(host)?;
        let methods = self.symbol_methods(type_symbol);
        let table = sub.clone().into();
        let method = methods.into_iter().fold(None, |candidate, m| {
            self.method_folder(name, candidate, *m, &table, &mut extra)
        })?;

        let ty = self.symbols.get(method).ty;
        let ty = sub.apply(&mut self.types, ty);

        Some((method, ty))
    }

    pub fn deref_type(&self, ty: types::TypeId) -> types::TypeId {
        match self.resolve(ty) {
            types::Type::Signal(t) => t.inner,
            types::Type::Listener(t) => t.inner,
            _ => ty,
        }
    }
    pub fn unwrap_type(&self, ty: types::TypeId) -> (types::TypeId, Substitutions) {
        let (host, generic_args) = match self.resolve(ty) {
            types::Type::Ref(r) => (r.inner, r.args),
            _ => (ty, vec![]),
        };
        let host_ty = self.resolve(host);
        let type_params = host_ty.as_params().unwrap_or(&[]);
        let sub = Substitutions::with_initial(type_params, &generic_args);
        (host, sub)
    }
    fn method_folder<F>(
        &self,
        name: &str,
        best: Option<MethodSymbolId>,
        current: MethodSymbolId,
        sub: &SubstitutionTable,
        extra: &mut F,
    ) -> Option<MethodSymbolId>
    where
        F: FnMut(&MethodSymbol) -> bool,
    {
        let symbol = self.symbols.get(current);
        if symbol.name() != name {
            return best;
        }
        let matches_generic = symbol.concreteness() == 0 || symbol.matches_substitutions(sub);
        if !matches_generic {
            return best;
        }
        if !extra(symbol) {
            return best;
        }
        match best {
            Some(c)
                if self.symbols.get::<MethodSymbolId>(c).concreteness() > symbol.concreteness() =>
            {
                Some(c)
            }
            _ => Some(current),
        }
    }

    pub fn check_assigned_type(
        &mut self,
        expected: types::TypeId,
        got: types::TypeId,
        got_immutable: bool,
        loc: Location,
    ) {
        let ok = self.can_be_assigned_to(got, expected, got_immutable);

        if !ok {
            let got = self.types.display(got);
            let expected = self.types.display(expected);
            let error = DiagnosticKind::WrongType { expected, got };
            self.error(error, loc);
        }
    }

    /// Return the visited type arguments and the initial substitution table.
    pub fn visit_type_args(
        &mut self,
        type_args: Option<Vec<ast::Type>>,
        expected_type_params: &[types::TypeParam],
        loc: Location,
    ) -> (Option<Vec<types::TypeId>>, Substitutions) {
        let Some(type_args) = type_args else {
            return (None, Substitutions::new());
        };

        if type_args.len() > expected_type_params.len() {
            let error = DiagnosticKind::TooManyParams {
                expected: expected_type_params.len(),
                got: type_args.len(),
            };
            self.error(error, loc);
        }

        let type_args = type_args
            .into_iter()
            .map(|t| self.visit_type(t))
            .collect::<Vec<_>>();

        let substitutions = Substitutions::with_initial(expected_type_params, &type_args);
        (Some(type_args), substitutions)
    }

    pub fn check_expression_against(
        &mut self,
        node: ast::Expression,
        expected: types::TypeId,
        substitutions: &mut Substitutions,
    ) -> Option<ir::Expression> {
        let loc = node.loc();
        let got = self.visit_expression(node)?;
        self.can_be_assigned_to(got.ty(), expected, true);
        let Some(ty) = self.infer(got.ty()) else {
            self.error(DiagnosticKind::CannotInferType, got.loc());
            return None;
        };
        let expected = self.infer(expected)?;
        substitutions.unify(self, expected, ty, loc);
        Some(got)
    }

    /// Get the concrete type arguments for an expression of generic type.
    pub fn infer_type_args(
        &mut self,
        type_symbol: &StructSymbol,
        concrete_id: types::TypeId,
    ) -> Substitutions {
        let expected = self.resolve(type_symbol.ty);
        let params = expected.as_params().unwrap_or(&[]);
        let args = match self.resolve(concrete_id) {
            types::Type::Ref(r) => r.args,
            _ => vec![],
        };
        debug_assert!(params.len() >= args.len());
        Substitutions::with_initial(&params, &args)
    }
}
