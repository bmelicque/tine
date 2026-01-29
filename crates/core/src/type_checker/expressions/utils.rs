use crate::{type_checker::TypeChecker, types::TypeId, DiagnosticKind, Location};

impl TypeChecker<'_> {
    pub fn check_assigned_type(&mut self, expected: TypeId, got: TypeId, loc: Location) {
        if !self.can_be_assigned_to(got, expected) {
            let got = self.session.display_type(got);
            let expected = self.session.display_type(expected);
            let error = DiagnosticKind::WrongType { expected, got };
            self.error(error, loc);
        }
    }
}
