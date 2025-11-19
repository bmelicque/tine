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
    aliases: HashMap<TypeId, String>,
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
            aliases: HashMap::new(),
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
    pub fn add_alias(&mut self, ty: TypeId, name: String) {
        self.aliases.insert(ty, name);
    }

    pub fn get(&self, id: TypeId) -> &Type {
        &self.arena[id as usize]
    }

    pub fn find_id(&self, ty: &Type) -> Option<TypeId> {
        self.lookup.get(ty).copied()
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

    pub fn display_type(&self, ty: TypeId) -> String {
        if let Some(name) = self.aliases.get(&ty) {
            return name.clone();
        }
        match &self.arena[ty as usize] {
            Type::Array(t) => {
                format!("[]{}", self.display_type(t.element))
            }
            Type::Boolean => "boolean".into(),
            Type::Duck(t) => {
                format!("~{}", self.display_type(t.like))
            }
            Type::Dynamic => "dynamic".into(),
            Type::Enum(t) => t
                .variants
                .iter()
                .map(|variant| format!("{}({})", variant.name, self.display_type(variant.def)))
                .collect::<Vec<_>>()
                .join(" | "),
            Type::Function(t) => {
                let params = t
                    .params
                    .iter()
                    .map(|p| self.display_type(*p))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({}) => {}", params, self.display_type(t.return_type))
            }
            Type::Generic(_) => "generic".into(),
            Type::Listener(t) => {
                format!("@{}", self.display_type(t.inner))
            }
            Type::Map(t) => {
                format!(
                    "{}#{}",
                    self.display_type(t.key),
                    self.display_type(t.value)
                )
            }
            Type::Number => "number".into(),
            Type::Option(t) => {
                format!("?{}", self.display_type(t.some))
            }
            Type::Param(t) => t.name.clone(),
            Type::Reference(t) => {
                format!("&{}", self.display_type(t.target))
            }
            Type::Result(t) => {
                if let Some(error) = &t.error {
                    format!("{}!{}", self.display_type(*error), self.display_type(t.ok))
                } else {
                    format!("!{}", self.display_type(t.ok))
                }
            }
            Type::SelfType => "Self".into(),
            Type::Signal(t) => {
                format!("${}", self.display_type(t.inner))
            }
            Type::String => "string".into(),
            Type::Struct(t) => {
                let fields = t
                    .fields
                    .iter()
                    .map(|field| format!("{} {}", field.name, self.display_type(field.def)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", fields)
            }
            Type::Trait(t) => {
                let methods = t
                    .methods
                    .iter()
                    .map(|method| format!("{} {}", method.name, self.display_type(method.def)))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(".({})", methods)
            }
            Type::Tuple(t) => {
                let elements = t
                    .elements
                    .iter()
                    .map(|e| self.display_type(*e))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({})", elements)
            }
            Type::Unit => "()".into(),
            Type::Unknown => "unknown".into(),
            Type::Void => "void".into(),
        }
    }

    pub fn import(&mut self, from: &TypeStore, id: TypeId) -> TypeId {
        match from.get(id) {
            Type::Array(t) => {
                let element = self.import(from, t.element);
                self.add(Type::Array(ArrayType { element }))
            }
            Type::Boolean => TypeStore::BOOLEAN,
            Type::Duck(t) => {
                let like = self.import(from, t.like);
                self.add(Type::Duck(DuckType { like }))
            }
            Type::Dynamic => TypeStore::DYNAMIC,
            Type::Enum(t) => {
                let variants: Vec<_> = t
                    .variants
                    .iter()
                    .map(|variant| Variant {
                        name: variant.name.clone(),
                        def: self.import(from, variant.def),
                    })
                    .collect();
                self.add(Type::Enum(EnumType {
                    id: self.get_next_id(),
                    variants,
                }))
            }
            Type::Function(t) => {
                let params: Vec<_> = t
                    .params
                    .iter()
                    .map(|param| self.import(from, *param))
                    .collect();
                let return_type = self.import(from, t.return_type);
                self.add(Type::Function(FunctionType {
                    params,
                    return_type,
                }))
            }
            Type::Generic(g) => self.add(Type::Generic(g.clone())),
            Type::Listener(t) => {
                let inner = self.import(from, t.inner);
                self.add(Type::Listener(ListenerType { inner }))
            }
            Type::Map(t) => {
                let key = self.import(from, t.key);
                let value = self.import(from, t.value);
                self.add(Type::Map(MapType { key, value }))
            }
            Type::Number => TypeStore::NUMBER,
            Type::Option(t) => {
                let some = self.import(from, t.some);
                self.add(Type::Option(OptionType { some }))
            }
            Type::Param(t) => self.add(Type::Param(t.clone())),
            Type::Reference(t) => {
                let target = self.import(from, t.target);
                self.add(Type::Reference(ReferenceType { target }))
            }
            Type::Result(t) => {
                let ok = self.import(from, t.ok);
                let error = t.error.map(|err| self.import(from, err));
                self.add(Type::Result(ResultType { ok, error }))
            }
            Type::SelfType => self.add(Type::SelfType),
            Type::Signal(t) => {
                let inner = self.import(from, t.inner);
                self.add(Type::Signal(SignalType { inner }))
            }
            Type::String => TypeStore::STRING,
            Type::Struct(t) => {
                let fields: Vec<_> = t
                    .fields
                    .iter()
                    .map(|field| StructField {
                        name: field.name.clone(),
                        def: self.import(from, field.def),
                        optional: field.optional,
                    })
                    .collect();
                self.add(Type::Struct(StructType {
                    id: self.get_next_id(),
                    fields,
                }))
            }
            Type::Trait(t) => {
                let methods: Vec<_> = t
                    .methods
                    .iter()
                    .map(|method| TraitMethod {
                        name: method.name.clone(),
                        def: self.import(from, method.def),
                    })
                    .collect();
                self.add(Type::Trait(TraitType { methods }))
            }
            Type::Tuple(t) => {
                let elements: Vec<_> = t.elements.iter().map(|e| self.import(from, *e)).collect();
                self.add(Type::Tuple(TupleType { elements }))
            }
            Type::Unit => TypeStore::UNIT,
            Type::Unknown => TypeStore::UNKNOWN,
            Type::Void => TypeStore::VOID,
        }
    }
}
