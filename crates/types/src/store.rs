use std::collections::HashMap;

use crate::types::{StructType, Type, TypeId, TypeParam};

#[derive(Debug, Default, Clone)]
pub struct TypeStore {
    arena: Vec<Type>,
    lookup: HashMap<Type, TypeId>,
    aliases: Vec<(TypeId, String)>,
}

impl TypeStore {
    pub const UNKNOWN: TypeId = 0;
    pub const UNIT: TypeId = 1;
    pub const DYNAMIC: TypeId = 2;
    pub const BOOLEAN: TypeId = 3;
    pub const STRING: TypeId = 4;
    pub const INTEGER: TypeId = 5;
    pub const FLOAT: TypeId = 6;
    pub const ELEMENT: TypeId = 7;
    pub const ARRAY_PARAM: TypeId = 8;
    pub const ARRAY: TypeId = 9;

    pub fn new() -> Self {
        let mut store = Self::default();
        store.add(Type::Unknown);
        store.add(Type::Unit);
        store.add(Type::Dynamic);
        store.add(Type::Boolean);
        store.add(Type::String);
        store.add(Type::Integer);
        store.add(Type::Float);
        // TODO: should be a trait
        let element = store.add(StructType {
            id: TypeStore::ELEMENT,
            // TODO: define fields & params
            params: vec![],
            fields: vec![],
        });
        store.add_alias(element, "Element".into());
        store.add_array();
        store
    }

    fn add_array(&mut self) {
        let param = TypeParam {
            name: "T".into(),
            id: Self::ARRAY_PARAM,
        };
        let param_id = self.add(param.clone());
        debug_assert_eq!(param_id, Self::ARRAY_PARAM);
        let array_id = self.add(StructType {
            id: Self::ARRAY,
            params: vec![param],
            fields: vec![],
        });
        debug_assert_eq!(array_id, Self::ARRAY);
    }

    pub fn get_next_id(&self) -> TypeId {
        self.arena.len() as TypeId
    }
    pub fn add(&mut self, ty: impl Into<Type>) -> TypeId {
        let ty = ty.into();
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
    pub fn add_unique(&mut self, ty: Type) -> TypeId {
        let ty = match ty {
            Type::Struct(mut st) => {
                st.id = self.get_next_id();
                Type::Struct(st)
            }
            Type::Enum(mut e) => {
                e.id = self.get_next_id();
                Type::Enum(e)
            }
            Type::Param(mut p) => {
                p.id = self.get_next_id();
                Type::Param(p)
            }
            ty => ty,
        };
        self.arena.push(ty);
        (self.arena.len() - 1) as TypeId
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

    pub fn has_property(&self, host: TypeId, property_name: &str) -> bool {
        match &self.arena[host as usize] {
            Type::Struct(st) => st
                .fields
                .iter()
                .find(|f| f.name == *property_name)
                .is_some(),
            _ => false,
        }
    }

    pub fn display(&self, ty: TypeId) -> String {
        display_type(self, ty)
    }
}

pub fn display_type(store: &TypeStore, ty: TypeId) -> String {
    if let Some(name) = store.get_alias(ty) {
        return name.clone();
    }
    display_raw_type(store, ty)
}

pub fn display_raw_type(store: &TypeStore, ty: TypeId) -> String {
    match &store.get(ty) {
        Type::Boolean => "bool".into(),
        Type::Dynamic => "dynamic".into(),
        Type::Enum(t) => t
            .variants
            .iter()
            .map(|variant| format!("{}({})", variant.name, display_type(store, variant.def)))
            .collect::<Vec<_>>()
            .join(" | "),
        Type::Float => "float".into(),
        Type::Function(t) => {
            let params = t
                .params
                .iter()
                .map(|p| display_type(store, *p))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({}) => {}", params, display_type(store, t.return_type))
        }
        Type::Generic(_) => "generic".into(),
        Type::Listener(t) => {
            format!("Computed<{}>", display_type(store, t.inner))
        }
        Type::Integer => "int".into(),
        Type::Param(t) => t.name.clone(),
        Type::Ref(t) => {
            if t.inner == TypeStore::ARRAY {
                return format!("{}[]", display_type(store, t.args[0]));
            }

            let args = t
                .args
                .iter()
                .map(|arg| display_type(store, *arg))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}<{}>", display_type(store, t.inner), args)
        }
        Type::Result(t) => {
            if let Some(error) = &t.error {
                format!(
                    "{}!{}",
                    display_type(store, *error),
                    display_type(store, t.ok)
                )
            } else {
                format!("!{}", display_type(store, t.ok))
            }
        }
        Type::SelfType => "Self".into(),
        Type::Signal(t) => {
            format!("${}", display_type(store, t.inner))
        }
        Type::String => "str".into(),
        Type::Struct(t) => {
            let fields = t
                .fields
                .iter()
                .map(|field| format!("{}: {}", field.name, display_type(store, field.def)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", fields)
        }
        Type::Trait(t) => {
            let methods = t
                .methods
                .iter()
                .map(|method| format!("{} {}", method.name, display_type(store, method.def)))
                .collect::<Vec<_>>()
                .join(", ");
            format!(".({})", methods)
        }
        Type::Tuple(t) => {
            let elements = t
                .elements
                .iter()
                .map(|e| display_type(store, *e))
                .collect::<Vec<_>>()
                .join(", ");
            format!("({})", elements)
        }
        Type::Unit => "()".into(),
        Type::Unknown => "unknown".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::*;

    #[test]
    fn test_typestore_initialization() {
        let store = TypeStore::new();
        assert_eq!(store.get(TypeStore::UNKNOWN), &Type::Unknown);
        assert_eq!(store.get(TypeStore::UNIT), &Type::Unit);
        assert_eq!(store.get(TypeStore::DYNAMIC), &Type::Dynamic);
        assert_eq!(store.get(TypeStore::BOOLEAN), &Type::Boolean);
        assert_eq!(store.get(TypeStore::STRING), &Type::String);
        assert_eq!(store.get(TypeStore::INTEGER), &Type::Integer);
        assert_eq!(store.get(TypeStore::FLOAT), &Type::Float);
    }

    #[test]
    fn test_has_property_field() {
        let mut store = TypeStore::new();
        let struct_type = Type::Struct(StructType {
            id: store.get_next_id(),
            params: vec![],
            fields: vec![StructField {
                name: "name".to_string(),
                def: TypeStore::STRING,
            }],
        });
        let struct_id = store.add(struct_type);

        assert!(store.has_property(struct_id, "name"));
        assert!(!store.has_property(struct_id, "nonexistent"));
    }

    #[test]
    fn test_display_type_primitives() {
        let store = TypeStore::new();
        assert_eq!(display_type(&store, TypeStore::INTEGER), "int");
        assert_eq!(display_type(&store, TypeStore::STRING), "str");
        assert_eq!(display_type(&store, TypeStore::BOOLEAN), "bool");
        assert_eq!(display_type(&store, TypeStore::UNIT), "()");
    }

    #[test]
    fn test_display_type_function() {
        let mut store = TypeStore::new();
        let fn_type = Type::Function(FunctionType {
            type_params: vec![],
            params: vec![TypeStore::STRING, TypeStore::INTEGER],
            return_type: TypeStore::BOOLEAN,
        });
        let fn_id = store.add(fn_type);
        assert_eq!(display_type(&store, fn_id), "(str, int) => bool");
    }

    #[test]
    fn test_add_alias() {
        let mut store = TypeStore::new();
        store.add_alias(TypeStore::INTEGER, "MyNumber".to_string());
        assert_eq!(display_type(&store, TypeStore::INTEGER), "MyNumber");
    }
}
