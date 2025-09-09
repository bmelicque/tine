use std::collections::HashMap;

use crate::types;

#[derive(Debug)]
pub struct VariableInfo {
    pub ty: types::Type,
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

    pub fn define(&mut self, name: &str, type_: types::Type, mutable: bool) {
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
    pub methods: HashMap<String, types::FunctionType>,
    pub loose_only: bool,
}

impl TypeMetadata {
    pub fn new() -> Self {
        Self {
            type_params: vec![],
            methods: HashMap::new(),
            loose_only: false,
        }
    }

    pub fn from_type_params(type_params: Vec<String>) -> Self {
        Self {
            type_params,
            methods: HashMap::new(),
            loose_only: false,
        }
    }
}

#[derive(Default)]
pub struct TypeRegistry {
    pub current_self: Option<String>,
    generics: HashMap<String, types::Type>,
    pub types: HashMap<String, types::Type>,
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

    pub fn create(names: &Vec<String>, types: &Vec<types::Type>) -> TypeRegistry {
        let mut registry = TypeRegistry::new();

        for (i, name) in names.iter().enumerate() {
            let t = types.get(i).cloned().unwrap_or(types::Type::Dynamic);
            registry.define(name, t, None);
        }

        registry
    }

    pub fn define(&mut self, name: &str, ty: types::Type, metadata: Option<TypeMetadata>) {
        self.types.insert(name.to_string(), ty);
        if let Some(data) = metadata {
            self.metadata.insert(name.into(), data);
        }
    }

    pub fn lookup(&self, name: &str) -> Option<types::Type> {
        if let Some(ref current_self) = self.current_self {
            if name == current_self {
                return Some(types::Type::SelfType);
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
        self.generics.insert(
            name.to_string(),
            types::Type::Generic(types::GenericType { name: name.into() }),
        );
    }

    pub fn clear_generics(&mut self) {
        self.generics.clear();
    }

    pub fn define_method(&mut self, on: types::NamedType, name: String, ty: types::FunctionType) {
        match self.metadata.get_mut(on.name.as_str()) {
            Some(data) => {
                data.methods.insert(name, ty);
            }
            None => {
                let mut data = TypeMetadata::new();
                data.methods.insert(name, ty);
            }
        };
    }

    pub fn type_has(&self, type_name: &str, name: &str) -> bool {
        let metadata = match self.metadata.get(type_name) {
            Some(data) => data,
            None => &TypeMetadata::new(),
        };
        if metadata.methods.get(name).is_some() {
            return true;
        }
        let ty = self.types.get(type_name).unwrap();
        let types::Type::Struct(ty) = ty else {
            return false;
        };
        ty.fields.iter().find(|field| field.name == name).is_some()
    }

    pub fn can_be_strict(&self, type_name: &str) -> bool {
        match self.metadata.get(type_name) {
            Some(data) => !data.loose_only,
            None => true,
        }
    }
}
