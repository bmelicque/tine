use std::collections::HashSet;

use crate::{
    ast,
    type_checker::{
        analysis_context::{type_store::TypeStore, SymbolData, SymbolRef},
        patterns::TokenList,
        TypeChecker,
    },
    types::{Type, TypeId, Variant},
    SymbolKind,
};

impl TypeChecker {
    pub fn visit_match_expression(&mut self, node: &ast::MatchExpression) -> TypeId {
        let ty = self.visit_expression(&node.scrutinee);
        let mut expected = TypeStore::DYNAMIC;
        for arm in &node.arms {
            let arm_ty = self.visit_match_arm(arm, ty, vec![]);
            if expected == TypeStore::DYNAMIC {
                expected = arm_ty;
            } else {
                self.check_assigned_type(expected, arm_ty, node.span);
            }
        }
        self.check_exhaustiveness(node, ty);
        self.analysis_context
            .save_expression_type(node.span, expected)
    }

    fn visit_match_arm(
        &mut self,
        arm: &ast::MatchArm,
        against: TypeId,
        deps: Vec<SymbolRef>,
    ) -> TypeId {
        let arm_ty = self.with_scope(arm.span, |s| {
            let mut variables = TokenList::new();
            s.match_pattern(&arm.pattern, against, &mut variables);
            for (name, ty) in variables.0 {
                let symbol = s.analysis_context.register_symbol(SymbolData {
                    name: name.as_str().into(),
                    kind: SymbolKind::constant(ty),
                    defined_at: arm.pattern.as_span(),
                    dependencies: deps.clone(),
                    ..Default::default()
                });
                s.analysis_context.save_symbol_token(name.span, symbol);
            }
            s.visit_expression(&arm.expression)
        });
        arm_ty
    }

    fn check_exhaustiveness(&mut self, node: &ast::MatchExpression, against: TypeId) {
        let has_irrefutable = node
            .arms
            .iter()
            .find(|arm| arm.pattern.is_identifier())
            .is_some();
        if has_irrefutable {
            return;
        }
        let against = self.resolve(against);
        match against.clone() {
            Type::Enum(ty) => self.check_variants_exhaustiveness(node, &ty.variants),
            ty => self.error(
                format!("Cannot match against type {} (not implemented yet)", ty),
                node.span,
            ),
        }
    }

    fn check_variants_exhaustiveness(
        &mut self,
        node: &ast::MatchExpression,
        variants: &Vec<Variant>,
    ) {
        let mut names = HashSet::new();
        for variant in variants {
            names.insert(variant.name.clone());
        }
        for arm in &node.arms {
            let ast::Pattern::Variant(ref variant) = *arm.pattern else {
                continue;
            };
            names.remove(&variant.name);
        }
        if names.len() > 0 {
            self.error("Missing match cases".into(), node.span)
        }
    }
}
