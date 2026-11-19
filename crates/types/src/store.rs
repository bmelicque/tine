use std::collections::HashMap;

use crate::types::{Placeholder, StructType, TraitMethod, Type, TypeId, TypeParam};

#[derive(Debug, Default, Clone)]
pub struct TypeStore {
    arena: Vec<Type>,
    lookup: HashMap<Type, TypeId>,
    aliases: Vec<(TypeId, String)>,
}

impl TypeStore {
    pub const UNKNOWN: TypeId = 0;
    pub const UNIT: TypeId = 1;
    pub const BOOLEAN: TypeId = 2;
    pub const STRING: TypeId = 3;
    pub const INTEGER: TypeId = 4;
    pub const FLOAT: TypeId = 5;
    pub const ELEMENT: TypeId = 6;
    pub const ARRAY_PARAM: TypeId = 7;
    pub const ARRAY: TypeId = 8;

    pub fn new() -> Self {
        let mut store = Self::default();
        store.add(Type::Unknown);
        store.add(Type::Unit);
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
            Type::Placeholder(mut p) => {
                p.id = self.get_next_id();
                Type::Placeholder(p)
            }
            ty => ty,
        };
        self.arena.push(ty);
        (self.arena.len() - 1) as TypeId
    }
    pub fn add_new_placeholder(&mut self) -> TypeId {
        self.add_unique(Type::Placeholder(Placeholder::default()))
    }
    pub fn add_alias(&mut self, ty: TypeId, name: String) {
        self.aliases.push((ty, name));
    }

    /// Overwrites the given target type with the provided one.
    ///
    /// This should be used only for replacing "placeholder" type params that
    /// have been used for building a self-referencing type.
    pub fn overwrite<T>(&mut self, at: TypeId, new: T)
    where
        T: Into<Type>,
    {
        let mut new = new.into();
        match &mut new {
            Type::Struct(t) => t.id = at,
            Type::Enum(t) => t.id = at,
            _ => {}
        }
        self.arena[at as usize] = new;
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
        Type::Enum(t) => t
            .variants
            .iter()
            .map(|variant| format!("{}({})", variant.name, display_type(store, variant.def)))
            .collect::<Vec<_>>()
            .join(" | "),
        Type::Float => "float".into(),
        Type::Function(t) => {
            let params = display_params(store, &t.params);
            let return_type = display_return_type(store, t.return_type);
            format!("fn({}){}", params, return_type)
        }
        Type::Generic(_) => "generic".into(),
        Type::Listener(t) => {
            format!("Computed<{}>", display_type(store, t.inner))
        }
        Type::Integer => "int".into(),
        Type::Param(t) => t.name.clone(),
        Type::Placeholder(_) => "{unknown}".into(),
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
                .map(|method| display_trait_method(store, method))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {} }}", methods)
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

fn display_params(store: &TypeStore, params: &[TypeId]) -> String {
    params
        .iter()
        .map(|p| display_type(store, *p))
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_return_type(store: &TypeStore, ty: TypeId) -> String {
    match ty {
        TypeStore::UNIT => "".to_string(),
        ty => format!(" -> {}", display_type(store, ty)),
    }
}

fn display_trait_method(store: &TypeStore, method: &TraitMethod) -> String {
    let name = &method.name;
    let Type::Function(ty) = store.get(method.def) else {
        panic!()
    };
    let params = display_params(store, &ty.params);
    let return_type = display_return_type(store, ty.return_type);
    format!("fn {}({}){}", name, params, return_type)
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
        assert_eq!(display_type(&store, fn_id), "fn(str, int) -> bool");
    }

    #[test]
    fn test_add_alias() {
        let mut store = TypeStore::new();
        store.add_alias(TypeStore::INTEGER, "MyNumber".to_string());
        assert_eq!(display_type(&store, TypeStore::INTEGER), "MyNumber");
    }
}
