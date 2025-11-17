use std::collections::HashMap;

use crate::types::{
    ArrayType, DuckType, EnumType, FunctionType, ListenerType, MapType, OptionType, ReferenceType,
    ResultType, SignalType, StructField, StructType, TraitMethod, TraitType, TupleType, Type,
    TypeId, Variant,
};

#[derive(Debug, Clone)]
pub struct TypeMetadata {
    pub methods: HashMap<String, TypeId>,
}
impl TypeMetadata {
    pub fn new() -> Self {
        TypeMetadata {
            methods: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeStore {
    arena: Vec<Type>,
    lookup: HashMap<Type, TypeId>,
    metadata: HashMap<TypeId, TypeMetadata>,
}

impl TypeStore {
    pub const UNKNOWN: TypeId = 0;
    pub const VOID: TypeId = 1;
    pub const UNIT: TypeId = 1;
    pub const DYNAMIC: TypeId = 3;
    pub const BOOLEAN: TypeId = 4;
    pub const STRING: TypeId = 5;
    pub const NUMBER: TypeId = 6;

    pub fn new() -> Self {
        let mut store = Self {
            arena: vec![],
            lookup: HashMap::new(),
            metadata: HashMap::new(),
        };
        store.add(Type::Unknown);
        store.add(Type::Void);
        store.add(Type::Unit);
        store.add(Type::Dynamic);
        store.add(Type::Boolean);
        store.add(Type::String);
        store.add(Type::Number);
        store
    }

    pub fn get_next_id(&self) -> TypeId {
        self.arena.len() as TypeId
    }
    pub fn add(&mut self, ty: Type) -> TypeId {
        match self.lookup.get(&ty) {
            Some(id) => *id,
            None => {
                let id = self.arena.len() as TypeId;
                self.arena.push(ty);
                id
            }
        }
    }

    pub fn get(&self, id: TypeId) -> &Type {
        &self.arena[id as usize]
    }

    pub(crate) fn define_method(&mut self, host: TypeId, method_name: &str, signature: TypeId) {
        if !self.metadata.contains_key(&host) {
            self.metadata.insert(host, TypeMetadata::new());
        }
        let metadata = self.metadata.get_mut(&host).unwrap();
        let r = metadata.methods.insert(method_name.to_string(), signature);
        if r.is_some() {
            panic!("method already exists, this should've been check before calling this");
        }
    }
    pub fn find_method(&self, host: TypeId, method_name: &str) -> Option<TypeId> {
        let Some(metadata) = self.metadata.get(&host) else {
            return None;
        };
        metadata.methods.get(method_name).copied()
    }
    pub fn has_property(&self, host: TypeId, property_name: &str) -> bool {
        if self.find_method(host, property_name).is_some() {
            return true;
        }
        match &self.arena[host as usize] {
            Type::Struct(st) => st
                .fields
                .iter()
                .find(|f| f.name == *property_name)
                .is_some(),
            _ => false,
        }
    }

    /// Canonicalize a type definition by replacing the type params with the provided arguments.
    ///
    /// For example, with definition `Box[T] :: ( value T )`, `Box[number]` will become `( value T )`
    pub fn substitute(&mut self, ty_id: TypeId, args: &[TypeId]) -> TypeId {
        let ty = self.get(ty_id).clone();
        match ty {
            Type::Array(t) => {
                let element = self.substitute(t.element, args);
                self.add(Type::Array(ArrayType { element }))
            }
            Type::Boolean => ty_id,
            Type::Duck(t) => {
                let like = self.substitute(t.like, args);
                self.add(Type::Duck(DuckType { like }))
            }
            Type::Dynamic => panic!("should not encounter dynamic typing during type substitution"),
            Type::Enum(t) => {
                let variants: Vec<_> = t
                    .variants
                    .iter()
                    .map(|variant| Variant {
                        name: variant.name.clone(),
                        def: self.substitute(variant.def, args),
                    })
                    .collect();
                self.add(Type::Enum(EnumType { id: t.id, variants }))
            }
            Type::Function(t) => {
                let params: Vec<_> = t
                    .params
                    .iter()
                    .map(|param| self.substitute(*param, args))
                    .collect();
                let return_type = self.substitute(t.return_type, args);
                self.add(Type::Function(FunctionType {
                    params,
                    return_type,
                }))
            }
            Type::Generic(_) => unimplemented!("cannot handle nested generics"),
            Type::Listener(t) => {
                let inner = self.substitute(t.inner, args);
                self.add(Type::Listener(ListenerType { inner }))
            }
            Type::Map(t) => {
                let key = self.substitute(t.key, args);
                let value = self.substitute(t.value, args);
                self.add(Type::Map(MapType { key, value }))
            }
            Type::Number => ty_id,
            Type::Option(t) => {
                let some = self.substitute(t.some, args);
                self.add(Type::Option(OptionType { some }))
            }
            Type::Param(t) => args[t.idx],
            Type::Reference(t) => {
                let target = self.substitute(t.target, args);
                self.add(Type::Reference(ReferenceType { target }))
            }
            Type::Result(t) => {
                let ok = self.substitute(t.ok, args);
                let error = t.error.map(|err| self.substitute(err, args));
                self.add(Type::Result(ResultType { ok, error }))
            }
            Type::SelfType => ty_id,
            Type::Signal(t) => {
                let inner = self.substitute(t.inner, args);
                self.add(Type::Signal(SignalType { inner }))
            }
            Type::String => ty_id,
            Type::Struct(t) => {
                let fields: Vec<_> = t
                    .fields
                    .iter()
                    .map(|field| StructField {
                        name: field.name.clone(),
                        def: self.substitute(field.def, args),
                        optional: field.optional,
                    })
                    .collect();
                self.add(Type::Struct(StructType { id: t.id, fields }))
            }
            Type::Trait(t) => {
                let methods: Vec<_> = t
                    .methods
                    .iter()
                    .map(|method| TraitMethod {
                        name: method.name.clone(),
                        def: self.substitute(method.def, args),
                    })
                    .collect();
                self.add(Type::Trait(TraitType { methods }))
            }
            Type::Tuple(t) => {
                let elements: Vec<_> = t
                    .elements
                    .iter()
                    .map(|e| self.substitute(*e, args))
                    .collect();
                self.add(Type::Tuple(TupleType { elements }))
            }
            Type::Unit => ty_id,
            Type::Unknown => ty_id,
            Type::Void => ty_id,
        }
    }

    /// Check if the `test` type implements given duck type.
    pub(crate) fn implements(&self, test: TypeId, duck: &DuckType) -> bool {
        if let Type::Duck(test) = &self.arena[test as usize] {
            if *test == *duck {
                return true;
            }
        }
        let like = self.arena[duck.like as usize].clone();
        let Type::Struct(ref st) = like else {
            return false;
        };

        st.fields
            .iter()
            .find(|field| !self.has_property(test, &field.name))
            .is_none()
    }

    pub fn find_id(&self, ty: &Type) -> Option<TypeId> {
        self.lookup.get(ty).copied()
    }
}
