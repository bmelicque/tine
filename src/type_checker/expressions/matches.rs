use std::collections::HashSet;

use crate::{
    ast,
    parser::parser::ParseError,
    type_checker::TypeChecker,
    types::{SumVariant, Type},
};

impl TypeChecker {
    pub fn visit_match_expression(&mut self, node: &ast::MatchExpression) -> Type {
        let ty = self.visit_expression(&node.scrutinee);
        let mut expected = Type::Dynamic;
        for arm in &node.arms {
            let arm_ty = self.visit_match_arm(arm, ty.clone());
            if expected == Type::Dynamic {
                expected = arm_ty;
            } else if !arm_ty.is_assignable_to(&expected) {
                self.errors.push(ParseError {
                    message: format!("Arm type mismatch: expected {}, got {}", expected, arm_ty),
                    span: node.span,
                });
            }
        }
        self.check_exhaustiveness(node, &ty);
        expected
    }

    fn visit_match_arm(&mut self, arm: &ast::MatchArm, against: Type) -> Type {
        let mut variables = Vec::new();
        self.match_pattern(&arm.pattern, against, &mut variables);
        self.symbols.enter_scope();
        for (name, ty) in variables {
            self.symbols.define(&name, ty, false);
        }
        let arm_ty = self.visit_expression(&arm.expression);
        self.symbols.exit_scope();
        arm_ty
    }

    fn check_exhaustiveness(&mut self, node: &ast::MatchExpression, against: &Type) {
        let has_irrefutable = node
            .arms
            .iter()
            .find(|arm| arm.pattern.is_identifier())
            .is_some();
        if has_irrefutable {
            return;
        }
        match self.unwrap_named_type(&against) {
            Type::Sum { variants } => self.check_variants_exhaustiveness(node, &variants),
            ty => self.errors.push(ParseError {
                message: format!("Cannot match against type {} (not implemented yet)", ty),
                span: node.span,
            }),
        }
    }

    fn check_variants_exhaustiveness(
        &mut self,
        node: &ast::MatchExpression,
        variants: &Vec<SumVariant>,
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
            self.errors.push(ParseError {
                message: "Missing match cases".into(),
                span: node.span,
            })
        }
    }
}
