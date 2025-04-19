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
    types: HashMap<String, Type>,
}

impl TypeRegistry {
    pub fn new() -> Self {
        Self {
            current_self: None,
            types: HashMap::new(),
        }
    }

    pub fn define(&mut self, name: &str, ty: Type) {
        self.types.insert(name.to_string(), ty);
    }

    pub fn lookup(&self, name: &str) -> Option<&Type> {
        match self.current_self {
            Some(ref current_self) if name == current_self => Some(&Type::SelfType),
            _ => self.types.get(name),
        }
    }
}
