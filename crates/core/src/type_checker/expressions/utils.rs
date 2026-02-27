use std::collections::HashMap;

use crate::{
    ast,
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

    pub fn get_explicit_substitutions(
        &mut self,
        type_args: &Option<Vec<ast::Type>>,
        expected_type_params: &[TypeId],
        loc: Location,
    ) -> HashMap<types::TypeParam, TypeId> {
        let mut substitutions = HashMap::new();
        if let Some(type_args) = type_args {
            if type_args.len() > expected_type_params.len() {
                let error = DiagnosticKind::TooManyParams {
                    expected: expected_type_params.len(),
                    got: type_args.len(),
                };
                self.error(error, loc);
            }

            for (param, arg) in expected_type_params.iter().zip(type_args) {
                let type_arg = self.visit_type(arg);
                if let types::Type::Param(p) = self.resolve(*param) {
                    substitutions.insert(p, type_arg);
                }
            }
        }
        substitutions
    }

    pub fn check_expression_against(
        &mut self,
        node: &ast::Expression,
        expected: TypeId,
        substitutions: &mut HashMap<types::TypeParam, TypeId>,
    ) {
        let got = self.visit_expression(node);
        self.unify(expected, got, node.loc(), substitutions);
    }
}
