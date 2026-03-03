use std::collections::HashSet;

use crate::{
    ast,
    diagnostics::DiagnosticKind,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData, SymbolRef},
        patterns::TokenList,
        TypeChecker,
    },
    types::{Type, TypeId, Variant},
    SymbolKind,
};

impl TypeChecker<'_> {
    pub fn visit_match_expression(&mut self, node: &ast::MatchExpression) -> TypeId {
        let ty = match &node.scrutinee {
            Some(scrutinee) => self.visit_expression(scrutinee),
            None => TypeStore::UNKNOWN,
        };
        let mut expected = TypeStore::DYNAMIC;
        if let Some(arms) = &node.arms {
            for arm in arms {
                let arm_ty = self.visit_match_arm(arm, ty, vec![]);
                if expected == TypeStore::DYNAMIC {
                    expected = arm_ty;
                } else {
                    self.check_assigned_type(expected, arm_ty, node.loc);
                }
            }
        }
        self.check_exhaustiveness(node, ty);
        self.ctx.save_expression_type(node.loc, expected)
    }

    fn visit_match_arm(
        &mut self,
        arm: &ast::MatchArm,
        against: TypeId,
        deps: Vec<SymbolRef>,
    ) -> TypeId {
        let arm_ty = self.with_scope(|s| {
            let mut variables = TokenList::new();
            if let Some(pattern) = &arm.pattern {
                s.match_pattern(pattern, against, &mut variables);
                for (name, ty) in variables.0 {
                    s.ctx.register_symbol(SymbolData {
                        name: name.as_str().into(),
                        ty,
                        kind: SymbolKind::constant(),
                        defined_at: pattern.loc(),
                        dependencies: deps.clone(),
                        ..Default::default()
                    });
                }
            }
            match &arm.expression {
                Some(e) => s.visit_expression(e),
                None => TypeStore::UNKNOWN,
            }
        });
        arm_ty
    }

    fn check_exhaustiveness(&mut self, node: &ast::MatchExpression, against_id: TypeId) {
        let Some(arms) = &node.arms else {
            return;
        };
        let has_irrefutable = arms
            .iter()
            .find(|arm| {
                arm.pattern
                    .as_ref()
                    .map(|p| p.is_identifier())
                    .unwrap_or(false)
            })
            .is_some();
        if has_irrefutable {
            return;
        }
        let against = self.resolve(against_id);
        match &against {
            Type::Enum(ty) => self.check_variants_exhaustiveness(node, &ty.variants),
            _ => self.error(
                DiagnosticKind::ExpectedEnum {
                    got: self.session.display_type(against_id),
                },
                node.loc,
            ),
        }
    }

    fn check_variants_exhaustiveness(
        &mut self,
        node: &ast::MatchExpression,
        variants: &Vec<Variant>,
    ) {
        let Some(arms) = &node.arms else {
            return;
        };
        let mut names = HashSet::new();
        for variant in variants {
            names.insert(variant.name.clone());
        }
        for arm in arms {
            // TODO
        }
        if names.len() > 0 {
            let error = DiagnosticKind::NonExhaustiveMatch {
                missing: names.into_iter().collect::<Vec<_>>(),
            };
            self.error(error, node.loc)
        }
    }
}
