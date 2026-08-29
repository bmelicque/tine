use tine_types::types::{self, Placeholder, TypeId};

use crate::TypeChecker;

impl TypeChecker {
    pub fn new_type_placeholder(&mut self) -> Placeholder {
        let ty = self.intern_unique(Placeholder::default());
        Placeholder { id: ty }
    }

    // pub fn solve_placeholders(&mut self, got: TypeId, hint: TypeId) -> Option<TypeId> {
    //     use types::Type::*;
    //     match (self.resolve(got), self.resolve(hint)) {
    //         Placeholder(p) => self.placeholders.get(&p).copied(),

    //         Function(f) => self.infer_funtion_type(f),
    //         Listener(l) => self.infer_listener_type(l),
    //         Ref(r) => self.infer_type_ref(r),
    //         Signal(s) => self.infer_signal_type(s),
    //         Tuple(t) => self.infer_tuple_type(t),
    //         _ => Some(ty),
    //     }
    // }

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
}
