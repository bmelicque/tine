use std::collections::HashMap;

use crate::types::{StructType, TraitMethod, TraitType, Type, TypeId};

#[derive(Debug, Default, Clone)]
pub struct TypeStore {
    arena: Vec<Type>,
    lookup: HashMap<Type, TypeId>,
    aliases: Vec<(TypeId, String)>,
    methods: HashMap<TypeId, Vec<TraitMethod>>,
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
        store
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
    pub fn add_method(&mut self, host: TypeId, method: TraitMethod) {
        let methods = self.methods.entry(host).or_insert_with(Vec::new);
        if !methods.contains(&method) {
            methods.push(method);
        }
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

    pub fn can_assign_to(&self, actual_id: TypeId, expected_id: TypeId) -> bool {
        let actual = self.get(actual_id);
        let expected = self.get(expected_id);
        match (&expected, &actual) {
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (Type::Trait(t), _) => self.implements(actual_id, t),
            (e, Type::Ref(a)) if e.is_generic() => a.inner == expected_id,
            (_, _) => actual == expected,
        }
    }

    /// Check if the `test` type implements given trait.
    pub(crate) fn implements(&self, test: TypeId, trait_: &TraitType) -> bool {
        if trait_.methods.is_empty() {
            return false;
        }
        if let Type::Trait(test) = &self.get(test) {
            if *test == *trait_ {
                return true;
            }
        }
        let Some(methods) = self.methods.get(&test) else {
            return false;
        };

        trait_
            .methods
            .iter()
            .find(|m| !methods.contains(m))
            .is_none()
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
    fn test_add_type() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::FLOAT,
        });
        let id = store.add(array_type.clone());
        assert_eq!(store.get(id), &array_type);
    }

    #[test]
    fn test_add_duplicate_type() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::FLOAT,
        });
        let id1 = store.add(array_type.clone());
        let id2 = store.add(array_type);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_find_id() {
        let mut store = TypeStore::new();
        let array_type = Type::Array(ArrayType {
            element: TypeStore::INTEGER,
        });
        let id = store.add(array_type.clone());
        assert_eq!(store.find_id(&array_type), Some(id));
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
}
