use tine_ast as ast;
use tine_common::{
    diagnostics::DiagnosticKind,
    locations::{Locatable, Location},
};
use tine_ir as ir;
use tine_symbols::symbols::StructSymbol;
use tine_types::types;

use crate::{substitutions::Substitutions, TypeChecker};

impl TypeChecker {
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
        let got = self.visit_expression(node);
        if let Some(got) = &got {
            substitutions.unify(self, expected, got.ty(), loc);
        }
        got
    }

    /// Get the concrete type arguments for an expression of generic type.
    pub fn infer_type_args(
        &mut self,
        type_symbol: &StructSymbol,
        concrete_id: types::TypeId,
    ) -> Substitutions {
        let params = match self.resolve(type_symbol.ty) {
            types::Type::Generic(g) => g.params,
            _ => vec![],
        };
        let args = match self.resolve(concrete_id) {
            types::Type::Ref(r) => r.args,
            _ => vec![],
        };
        debug_assert!(params.len() >= args.len());
        Substitutions::with_initial(&params, &args)
    }
}
