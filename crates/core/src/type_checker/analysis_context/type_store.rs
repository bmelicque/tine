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
    aliases: Vec<(TypeId, String)>,
}

impl TypeStore {
    pub const UNKNOWN: TypeId = 0;
    pub const VOID: TypeId = 1;
    pub const UNIT: TypeId = 2;
    pub const DYNAMIC: TypeId = 3;
    pub const BOOLEAN: TypeId = 4;
    pub const STRING: TypeId = 5;
    pub const NUMBER: TypeId = 6;
    pub const ELEMENT: TypeId = 7;

    pub fn new() -> Self {
        let mut store = Self {
            arena: vec![],
            lookup: HashMap::new(),
            metadata: HashMap::new(),
            aliases: Vec::new(),
        };
        store.add(Type::Unknown);
        store.add(Type::Void);
        store.add(Type::Unit);
        store.add(Type::Dynamic);
        store.add(Type::Boolean);
        store.add(Type::String);
        store.add(Type::Number);
        let element = store.add(Type::Struct(StructType {
            id: TypeStore::ELEMENT,
            // TODO: define fields
            fields: vec![],
        }));
        store.add_alias(element, "Element".into());
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
                self.arena.push(ty.clone());
                self.lookup.insert(ty, id);
                id
            }
        }
    }
    pub fn add_alias(&mut self, ty: TypeId, name: String) {
        self.aliases.push((ty, name));
    }

    pub fn get(&self, id: TypeId) -> &Type {
        &self.arena[id as usize]
    }
    pub fn get_alias(&self, id: TypeId) -> Option<&String> {
        self.aliases.iter().find(|(i, _)| *i == id).map(|(_, a)| a)
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
        if let Some(name) = self.get_alias(ty) {
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
        let type_id = match from.get(id) {
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
        };
        if let Some(alias) = from.get_alias(id) {
            self.add_alias(type_id, alias.clone());
        }
        type_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typestore_initialization() {
        let store = TypeStore::new();
        assert_eq!(store.get(TypeStore::UNKNOWN), &Type::Unknown);
        assert_eq!(store.get(TypeStore::VOID), &Type::Void);
        assert_eq!(store.get(TypeStore::UNIT), &Type::Unit);
        assert_eq!(store.get(TypeStore::DYNAMIC), &Type::Dynamic);
        assert_eq!(store.get(TypeStore::BOOLEAN), &Type::Boolean);
        assert_eq!(store.get(TypeStore::STRING), &Type::String);
        assert_eq!(store.get(TypeStore::NUMBER), &Type::Number);
    }

    #[test]
    fn test_add_type() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::NUMBER,
        });
        let id = store.add(array_type.clone());
        assert_eq!(store.get(id), &array_type);
    }

    #[test]
    fn test_add_duplicate_type() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::NUMBER,
        });
        let id1 = store.add(array_type.clone());
        let id2 = store.add(array_type);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_find_id() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::NUMBER,
        });
        let id = store.add(array_type.clone());
        assert_eq!(store.find_id(&array_type), Some(id));
    }

    #[test]
    fn test_add_alias() {
        let mut store = TypeStore::new();
        store.add_alias(TypeStore::NUMBER, "MyNumber".to_string());
        assert_eq!(store.display_type(TypeStore::NUMBER), "MyNumber");
    }

    #[test]
    fn test_define_and_find_method() {
        let mut store = TypeStore::new();
        let struct_type = Type::Struct(StructType {
            id: store.get_next_id(),
            fields: vec![],
        });
        let struct_id = store.add(struct_type);

        let fn_type = Type::Function(FunctionType {
            params: vec![],
            return_type: TypeStore::VOID,
        });
        let fn_id = store.add(fn_type);

        store.define_method(struct_id, "toString", fn_id);
        assert_eq!(store.find_method(struct_id, "toString"), Some(fn_id));
        assert_eq!(store.find_method(struct_id, "nonexistent"), None);
    }

    #[test]
    fn test_has_property_method() {
        let mut store = TypeStore::new();
        let struct_type = Type::Struct(StructType {
            id: store.get_next_id(),
            fields: vec![],
        });
        let struct_id = store.add(struct_type);

        let fn_type = Type::Function(FunctionType {
            params: vec![],
            return_type: TypeStore::VOID,
        });
        let fn_id = store.add(fn_type);

        store.define_method(struct_id, "toString", fn_id);
        assert!(store.has_property(struct_id, "toString"));
        assert!(!store.has_property(struct_id, "nonexistent"));
    }

    #[test]
    fn test_has_property_field() {
        let mut store = TypeStore::new();
        let struct_type = Type::Struct(StructType {
            id: store.get_next_id(),
            fields: vec![StructField {
                name: "name".to_string(),
                def: TypeStore::STRING,
                optional: false,
            }],
        });
        let struct_id = store.add(struct_type);

        assert!(store.has_property(struct_id, "name"));
        assert!(!store.has_property(struct_id, "nonexistent"));
    }

    #[test]
    fn test_substitute_array() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::NUMBER,
        });
        let array_id = store.add(array_type);
        let substituted = store.substitute(array_id, &[]);
        assert_eq!(
            store.get(substituted),
            &Type::Array(ArrayType {
                element: TypeStore::NUMBER,
            })
        );
    }

    #[test]
    fn test_substitute_function() {
        let mut store = TypeStore::new();
        let fn_type = Type::Function(FunctionType {
            params: vec![TypeStore::NUMBER, TypeStore::STRING],
            return_type: TypeStore::BOOLEAN,
        });
        let fn_id = store.add(fn_type.clone());
        let substituted = store.substitute(fn_id, &[]);
        assert_eq!(store.get(substituted), &fn_type);
    }

    #[test]
    fn test_import_array_type() {
        let mut source_store = TypeStore::new();
        let mut target_store = TypeStore::new();

        let array_type = Type::Array(ArrayType {
            element: TypeStore::NUMBER,
        });
        let array_id = source_store.add(array_type.clone());

        let imported_id = target_store.import(&source_store, array_id);
        assert_eq!(target_store.get(imported_id), &array_type);
    }

    #[test]
    fn test_import_struct_type() {
        let mut source_store = TypeStore::new();
        let mut target_store = TypeStore::new();

        let struct_type = Type::Struct(StructType {
            id: source_store.get_next_id(),
            fields: vec![
                StructField {
                    name: "name".to_string(),
                    def: TypeStore::STRING,
                    optional: false,
                },
                StructField {
                    name: "age".to_string(),
                    def: TypeStore::NUMBER,
                    optional: false,
                },
            ],
        });
        let struct_id = source_store.add(struct_type.clone());

        let imported_id = target_store.import(&source_store, struct_id);
        match target_store.get(imported_id) {
            Type::Struct(st) => {
                assert_eq!(st.fields.len(), 2);
                assert_eq!(st.fields[0].name, "name");
                assert_eq!(st.fields[0].def, TypeStore::STRING);
                assert_eq!(st.fields[1].name, "age");
                assert_eq!(st.fields[1].def, TypeStore::NUMBER);
            }
            _ => panic!("Expected struct type"),
        }
    }

    #[test]
    fn test_import_function_type() {
        let mut source_store = TypeStore::new();
        let mut target_store = TypeStore::new();

        let fn_type = Type::Function(FunctionType {
            params: vec![TypeStore::STRING, TypeStore::NUMBER],
            return_type: TypeStore::BOOLEAN,
        });
        let fn_id = source_store.add(fn_type.clone());

        let imported_id = target_store.import(&source_store, fn_id);
        assert_eq!(target_store.get(imported_id), &fn_type);
    }

    #[test]
    fn test_import_enum_type() {
        let mut source_store = TypeStore::new();
        let mut target_store = TypeStore::new();

        let enum_type = Type::Enum(EnumType {
            id: source_store.get_next_id(),
            variants: vec![
                Variant {
                    name: "Some".to_string(),
                    def: TypeStore::NUMBER,
                },
                Variant {
                    name: "None".to_string(),
                    def: TypeStore::UNIT,
                },
            ],
        });
        let enum_id = source_store.add(enum_type.clone());

        let imported_id = target_store.import(&source_store, enum_id);
        match target_store.get(imported_id) {
            Type::Enum(et) => {
                assert_eq!(et.variants.len(), 2);
                assert_eq!(et.variants[0].name, "Some");
                assert_eq!(et.variants[1].name, "None");
            }
            _ => panic!("Expected enum type"),
        }
    }

    #[test]
    fn test_import_nested_array() {
        let mut source_store = TypeStore::new();
        let mut target_store = TypeStore::new();

        let inner_array = Type::Array(ArrayType {
            element: TypeStore::NUMBER,
        });
        let inner_array_id = source_store.add(inner_array);

        let outer_array = Type::Array(ArrayType {
            element: inner_array_id,
        });
        let outer_array_id = source_store.add(outer_array);

        let imported_id = target_store.import(&source_store, outer_array_id);
        match target_store.get(imported_id) {
            Type::Array(at) => match target_store.get(at.element) {
                Type::Array(inner_at) => {
                    assert_eq!(inner_at.element, TypeStore::NUMBER);
                }
                _ => panic!("Expected nested array"),
            },
            _ => panic!("Expected array type"),
        }
    }

    #[test]
    fn test_import_with_alias() {
        let mut source_store = TypeStore::new();
        let mut target_store = TypeStore::new();

        let custom_type = Type::Struct(StructType {
            id: source_store.get_next_id(),
            fields: vec![],
        });
        let custom_id = source_store.add(custom_type);
        source_store.add_alias(custom_id, "MyStruct".to_string());

        let imported_id = target_store.import(&source_store, custom_id);
        assert_eq!(target_store.display_type(imported_id), "MyStruct");
    }

    #[test]
    fn test_display_type_primitives() {
        let store = TypeStore::new();
        assert_eq!(store.display_type(TypeStore::NUMBER), "number");
        assert_eq!(store.display_type(TypeStore::STRING), "string");
        assert_eq!(store.display_type(TypeStore::BOOLEAN), "boolean");
        assert_eq!(store.display_type(TypeStore::VOID), "void");
        assert_eq!(store.display_type(TypeStore::UNIT), "()");
    }

    #[test]
    fn test_display_type_array() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::NUMBER,
        });
        let array_id = store.add(array_type);
        assert_eq!(store.display_type(array_id), "[]number");
    }

    #[test]
    fn test_display_type_function() {
        let mut store = TypeStore::new();
        let fn_type = Type::Function(FunctionType {
            params: vec![TypeStore::STRING, TypeStore::NUMBER],
            return_type: TypeStore::BOOLEAN,
        });
        let fn_id = store.add(fn_type);
        assert_eq!(store.display_type(fn_id), "(string, number) => boolean");
    }
}
