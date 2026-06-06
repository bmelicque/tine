use tine_core::{types, Session};

pub struct SemanticsChecker<'a>(&'a Session);

impl<'a> SemanticsChecker<'a> {
    pub fn new(session: &'a Session) -> Self {
        SemanticsChecker(session)
    }

    /// Check if the given type implements Copy semantics.
    pub fn is_copy(&self, ty: types::TypeId) -> bool {
        let ty = self.0.get_type(ty);
        match ty {
            types::Type::Boolean
            | types::Type::Float
            | types::Type::Integer
            | types::Type::String
            | types::Type::Signal(_)
            | types::Type::Listener(_) => true,
            _ => false,
        }
    }

    pub fn is_trait(&self, ty: types::TypeId) -> bool {
        let ty = self.0.get_type(ty);
        matches!(ty, types::Type::Trait(_))
    }
}
