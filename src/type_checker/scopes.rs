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

pub struct TypeRegistry {
    pub current_self: Option<String>,
    pub current_type_params: Option<Vec<String>>,
    types: HashMap<String, Type>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            current_self: None,
            current_type_params: None,
            types: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: &str, ty: Type) {
        self.types.insert(name.to_string(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<Type> {
        if let Some(ref current_self) = self.current_self {
            if name == current_self {
                return Some(Type::SelfType);
            }
        }
        if let Some(ref current_type_params) = self.current_type_params {
            if current_type_params.contains(&name.to_string()) {
                return Some(Type::GenericParam(name.to_string()));
            }
        };
        self.types.get(name).cloned()
    }
}
