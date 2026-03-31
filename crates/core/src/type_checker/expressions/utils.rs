use std::collections::HashMap;

use crate::{
    ast, ir,
    type_checker::TypeChecker,
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
        expected_type_params: &[TypeId],
        loc: Location,
    ) -> (Option<Vec<TypeId>>, HashMap<types::TypeParam, TypeId>) {
        let Some(type_args) = type_args else {
            return (None, HashMap::new());
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

        let mut substitutions = HashMap::new();
        for (param, type_arg) in expected_type_params.iter().zip(&type_args) {
            if let types::Type::Param(p) = self.resolve(*param) {
                substitutions.insert(p, *type_arg);
            }
        }
        (Some(type_args), substitutions)
    }

    pub fn check_expression_against(
        &mut self,
        node: ast::Expression,
        expected: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) -> Option<ir::Expression> {
        let loc = node.loc();
        let got = self.visit_expression(node);
        if let Some(got) = &got {
            self.unify(expected, got.ty(), loc, substitutions);
        }
        got
    }
}
