use tine_common::diagnostics::DiagnosticKind;
use tine_types::{
    store::display_type,
    types::{self, Placeholder, TypeId},
};

use crate::TypeChecker;

impl TypeChecker {
    pub fn new_type_placeholder(&mut self) -> Placeholder {
        let ty = self.intern_unique(Placeholder::default());
        Placeholder { id: ty }
    }

    pub fn infer(&mut self, ty: TypeId) -> Option<TypeId> {
        use types::Type::*;
        match self.resolve(ty) {
            Placeholder(p) => self.placeholders.get(&p).copied(),

            Function(f) => self.infer_funtion_type(f),
            Listener(l) => self.infer_listener_type(l),
            Ref(r) => self.infer_type_ref(r),
            Signal(s) => self.infer_signal_type(s),
            Tuple(t) => self.infer_tuple_type(t),
            _ => Some(ty),
        }
    }
    fn infer_funtion_type(&mut self, ty: types::FunctionType) -> Option<TypeId> {
        let params = ty
            .params
            .into_iter()
            .map(|p| self.infer(p))
            .collect::<Option<Vec<_>>>()?;
        let return_type = self.infer(ty.return_type)?;
        let ty = self.intern(types::FunctionType {
            type_params: ty.type_params,
            params,
            return_type,
        });
        Some(ty)
    }
    fn infer_listener_type(&mut self, ty: types::ListenerType) -> Option<TypeId> {
        let inner = self.infer(ty.inner)?;
        let ty = self.intern(types::ListenerType { inner });
        Some(ty)
    }
    fn infer_type_ref(&mut self, ty: types::TypeRef) -> Option<TypeId> {
        let args = ty
            .args
            .into_iter()
            .map(|a| self.infer(a))
            .collect::<Option<Vec<_>>>()?;
        let ty = self.intern(types::TypeRef {
            inner: ty.inner,
            args,
        });
        Some(ty)
    }
    fn infer_signal_type(&mut self, ty: types::SignalType) -> Option<TypeId> {
        let inner = self.infer(ty.inner)?;
        let ty = self.intern(types::SignalType { inner });
        Some(ty)
    }
    fn infer_tuple_type(&mut self, ty: types::TupleType) -> Option<TypeId> {
        let elements = ty
            .elements
            .into_iter()
            .map(|e| self.infer(e))
            .collect::<Option<Vec<_>>>()?;
        let ty = self.intern(types::TupleType {
            params: ty.params,
            elements,
        });
        Some(ty)
    }

    pub fn unify_placeholders(
        &mut self,
        a: types::TypeId,
        b: types::TypeId,
    ) -> Result<(), DiagnosticKind> {
        use types::Type::*;
        let resolved_a = self.resolve(a);
        let resolved_b = self.resolve(b);
        match (resolved_a, resolved_b) {
            (Placeholder(p), _) => self.add_placeholder_constraint(p, b),
            (_, Placeholder(p)) => self.add_placeholder_constraint(p, a),

            (Function(a), Function(b)) if a.params.len() == b.params.len() => {
                self.unify_function_placeholders(a, b)
            }
            (Listener(a), Listener(b)) => self.unify_placeholders(a.inner, b.inner),
            (Ref(a), Ref(b)) if a.inner == b.inner => self.unify_ref_placeholders(a, b),
            (Signal(a), Signal(b)) => self.unify_placeholders(a.inner, b.inner),
            (Tuple(a), Tuple(b)) => self.unify_tuple_placeholders(a, b),
            (a, b) if a == b => Ok(()),
            _ => {
                let left_name = display_type(&self.types, a);
                let right_name = display_type(&self.types, b);
                Err(DiagnosticKind::MismatchedTypes {
                    left_name,
                    right_name,
                })
            }
        }
    }
    fn add_placeholder_constraint(
        &mut self,
        p: types::Placeholder,
        ty: types::TypeId,
    ) -> Result<(), DiagnosticKind> {
        if let types::Type::Param(_) = self.resolve(ty) {
            return Ok(());
        }
        match self.placeholders.get(&p) {
            Some(existing) => self.unify_placeholders(ty, *existing),
            None => {
                self.placeholders.insert(p, ty);
                Ok(())
            }
        }
    }
    fn unify_function_placeholders(
        &mut self,
        a: types::FunctionType,
        b: types::FunctionType,
    ) -> Result<(), DiagnosticKind> {
        for (a, b) in a.params.into_iter().zip(b.params) {
            self.unify_placeholders(a, b)?;
        }
        self.unify_placeholders(a.return_type, b.return_type)
    }
    fn unify_ref_placeholders(
        &mut self,
        a: types::TypeRef,
        b: types::TypeRef,
    ) -> Result<(), DiagnosticKind> {
        for (a, b) in a.args.into_iter().zip(b.args) {
            self.unify_placeholders(a, b)?;
        }
        Ok(())
    }
    fn unify_tuple_placeholders(
        &mut self,
        a: types::TupleType,
        b: types::TupleType,
    ) -> Result<(), DiagnosticKind> {
        for (a, b) in a.elements.into_iter().zip(b.elements) {
            self.unify_placeholders(a, b)?;
        }
        Ok(())
    }
}
