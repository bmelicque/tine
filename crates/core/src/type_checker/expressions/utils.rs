use crate::{type_checker::TypeChecker, types::TypeId, Location};

impl TypeChecker<'_> {
    pub fn check_assigned_type(&mut self, expected: TypeId, got: TypeId, loc: Location) {
        if !self.can_be_assigned_to(got, expected) {
            let got = self.ctx.type_store.display_type(got);
            let expected = self.ctx.type_store.display_type(expected);
            self.error(format!("Expected type {}, got {}", expected, got), loc);
        }
    }
}
