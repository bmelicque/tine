use std::collections::HashMap;

use crate::{store::TypeStore, types::*};

type Substitutions = HashMap<TypeParam, TypeId>;

impl TypeStore {
    /// Cut off excess arguments or add type placeholder so that args list
    /// matches the generic's params length.
    pub fn complete_type_args(&mut self, generic: TypeId, mut args: Vec<TypeId>) -> Vec<TypeId> {
        let def = self.get(generic);
        let params = def.as_params().unwrap_or(&[]);
        if args.len() >= params.len() {
            args.truncate(params.len());
            return args;
        }

        args.resize_with(params.len(), || self.add_new_placeholder());
        args
    }

    /// Instantiates a given generic type with the given type arguments.
    ///
    /// The caller should provide the correct amount of arguments, as any
    /// discrepancy will create a panic.
    pub fn instantiate(&mut self, generic: TypeId, args: &[TypeId]) -> TypeId {
        let def = self.get(generic);
        let params = def.as_params().unwrap_or(&[]);
        debug_assert_eq!(args.len(), params.len(), "expected as many args as params");
        let substitutions = params
            .into_iter()
            .zip(args)
            .map(|(param, &arg)| (param.clone(), arg))
            .collect::<Substitutions>();
        self.apply(generic, &substitutions)
    }

    fn apply(&mut self, type_id: TypeId, substitutions: &Substitutions) -> TypeId {
        use crate::types::Type::*;
        let ty = self.get(type_id).clone();
        match ty {
            Boolean | Float | Placeholder(_) | Integer | SelfType | String | Unit | Unknown => {
                type_id
            }
            Enum(ty) => self.apply_enum(ty, substitutions),
            Function(ty) => self.apply_function(ty, substitutions),
            Generic(ty) => self.apply(ty.def, substitutions),
            Param(ty) => self.apply_param(ty, substitutions),
            Listener(ty) => self.apply_listener(ty, substitutions),
            Ref(ty) => self.apply_ref(ty, substitutions),
            Result(ty) => self.apply_result(ty, substitutions),
            Signal(ty) => self.apply_signal(ty, substitutions),
            Struct(ty) => self.apply_struct(ty, substitutions),
            Trait(ty) => self.apply_trait(ty, type_id, substitutions),
            Tuple(ty) => self.apply_tuple(ty, substitutions),
        }
    }

    fn apply_enum(&mut self, ty: EnumType, substitutions: &Substitutions) -> TypeId {
        let args = ty
            .params
            .into_iter()
            .map(|p| *substitutions.get(&p).unwrap())
            .collect();
        self.add(TypeRef { inner: ty.id, args })
    }

    fn apply_function(&mut self, ty: FunctionType, substitutions: &Substitutions) -> TypeId {
        let params = ty
            .params
            .into_iter()
            .map(|p| self.apply(p, substitutions))
            .collect();
        let return_type = self.apply(ty.return_type, substitutions);
        self.add(FunctionType {
            type_params: vec![],
            params,
            return_type,
        })
    }

    fn apply_param(&mut self, ty: TypeParam, substitutions: &Substitutions) -> TypeId {
        substitutions
            .get(&ty)
            .copied()
            .expect("all type params should've been set")
    }

    fn apply_listener(&mut self, ty: ListenerType, substitutions: &Substitutions) -> TypeId {
        let inner = self.apply(ty.inner, substitutions);
        self.add(ListenerType { inner })
    }

    fn apply_ref(&mut self, ty: TypeRef, substitutions: &Substitutions) -> TypeId {
        let args = ty
            .args
            .into_iter()
            .map(|a| self.apply(a, substitutions))
            .collect::<Vec<_>>();
        self.instantiate(ty.inner, &args)
    }

    fn apply_result(&mut self, ty: ResultType, substitutions: &Substitutions) -> TypeId {
        let ok = self.apply(ty.ok, substitutions);
        let error = ty.error.map(|error| self.apply(error, substitutions));
        self.add(ResultType { ok, error })
    }

    fn apply_signal(&mut self, ty: SignalType, substitutions: &Substitutions) -> TypeId {
        let inner = self.apply(ty.inner, substitutions);
        self.add(SignalType { inner })
    }

    fn apply_struct(&mut self, ty: StructType, substitutions: &Substitutions) -> TypeId {
        let args = ty
            .params
            .into_iter()
            .map(|p| *substitutions.get(&p).unwrap())
            .collect();
        self.add(TypeRef { inner: ty.id, args })
    }

    fn apply_trait(&mut self, ty: TraitType, id: TypeId, substitutions: &Substitutions) -> TypeId {
        let args = ty
            .params
            .into_iter()
            .map(|p| *substitutions.get(&p).unwrap())
            .collect();
        self.add(TypeRef { inner: id, args })
    }

    fn apply_tuple(&mut self, ty: TupleType, substitutions: &Substitutions) -> TypeId {
        let elements = ty
            .elements
            .into_iter()
            .map(|e| self.apply(e, substitutions))
            .collect();
        self.add(TupleType {
            params: vec![],
            elements,
        })
    }
}
