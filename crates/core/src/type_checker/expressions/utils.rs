use crate::{
    ast, ir,
    type_checker::{substitutions::Substitutions, TypeChecker},
    types::{self, TypeId},
    DiagnosticKind, Location,
};

impl TypeChecker<'_> {
    pub fn check_assigned_type(&mut self, expected: TypeId, got: TypeId, loc: Location) {
        if !self.can_be_assigned_to(got, expected) {
            let got = self.session.display_type(got);
            let expected = self.session.display_type(expected);
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
    ) -> (Option<Vec<TypeId>>, Substitutions) {
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
        expected: TypeId,
        substitutions: &mut Substitutions,
    ) -> Option<ir::Expression> {
        let loc = node.loc();
        let got = self.visit_expression(node);
        if let Some(got) = &got {
            substitutions.unify(&mut self.session.types(), expected, got.ty(), loc);
        }
        got
    }
}
