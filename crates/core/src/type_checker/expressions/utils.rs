use pest::Span;

use crate::{type_checker::TypeChecker, types::TypeId};

impl TypeChecker {
    pub fn check_assigned_type(&mut self, expected: TypeId, got: TypeId, span: Span<'static>) {
        if !self.can_be_assigned_to(got, expected) {
            let got = self.analysis_context.type_store.display_type(got);
            let expected = self.analysis_context.type_store.display_type(expected);
            self.error(format!("Expected type {}, got {}", expected, got), span);
        }
    }
}
