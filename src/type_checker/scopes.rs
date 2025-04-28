use std::collections::HashMap;

use crate::types::Type;

pub struct VariableInfo {
    pub ty: Type,
    pub mutable: bool,
}

#[derive(Default)]
pub struct SymbolTable {
    scopes: Vec<HashMap<String, VariableInfo>>,
}

impl SymbolTable {
    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    pub fn exit_scope(&mut self) {
        self.scopes.pop().expect("Cannot pop the global scope");
    }

    pub fn define(&mut self, name: &str, type_: Type, mutable: bool) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_string(), VariableInfo { ty: type_, mutable });
        }
    }

    pub fn lookup(&self, name: &str) -> Option<&VariableInfo> {
        for scope in self.scopes.iter().rev() {
            if let Some(info) = scope.get(name) {
                return Some(info);
            }
        }
        None
    }
}

pub struct TypeMetadata {
    pub type_params: Vec<String>,
}

pub struct TypeRegistry {
    pub current_self: Option<String>,
    generics: HashMap<String, Type>,
    pub types: HashMap<String, Type>,
    metadata: HashMap<String, TypeMetadata>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            current_self: None,
            generics: HashMap::new(),
            types: HashMap::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn create(names: &Vec<String>, types: &Vec<Type>) -> TypeRegistry {
        let mut registry = TypeRegistry::new();

        for (i, name) in names.iter().enumerate() {
            let t = types.get(i).cloned().unwrap_or(Type::Dynamic);
            registry.define(name, t, None);
        }

        registry
    }

    pub fn define(&mut self, name: &str, ty: Type, metadata: Option<TypeMetadata>) {
        self.types.insert(name.to_string(), ty);
        if let Some(data) = metadata {
            self.metadata.insert(name.into(), data);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<Type> {
        if let Some(ref current_self) = self.current_self {
            if name == current_self {
                return Some(Type::SelfType);
            }
        }
        self.generics.get(name).or(self.types.get(name)).cloned()
    }

    pub fn get_type_params(&self, name: &str) -> Vec<String> {
        self.metadata
            .get(name)
            .map(|data| data.type_params.clone())
            .unwrap_or(Vec::new())
    }

    pub fn define_generic(&mut self, name: &str) {
        self.generics
            .insert(name.to_string(), Type::Generic(name.into()));
    }

    pub fn clear_generics(&mut self) {
        self.generics.clear();
    }
}
