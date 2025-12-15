mod symbols;
pub mod type_store;

pub use symbols::{SymbolData, SymbolHandle, SymbolKind, SymbolRef};

use std::collections::HashMap;

use crate::{
    locations::Span, type_checker::analysis_context::type_store::TypeStore, types::TypeId, Location,
};

#[derive(Clone, Debug)]
pub enum Token {
    Symbol(SymbolToken),
    Member(MemberToken),
}

#[derive(Clone, Debug)]
pub struct SymbolToken {
    pub span: Span,
    pub symbol: SymbolRef,
}

#[derive(Clone, Debug)]
pub struct MemberToken {
    pub span: Span,
    pub ty: TypeId,
}

#[derive(Clone)]
pub struct Scope {
    pub bindings: Vec<SymbolRef>,
    /**
     * Lists symbols captured by the scope
     */
    captured: Vec<SymbolRef>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            bindings: vec![],
            captured: vec![],
        }
    }

    pub fn has(&self, symbol: &SymbolRef) -> bool {
        self.bindings.iter().find(|b| b.is(symbol)).is_some()
    }

    pub fn capture(&mut self, symbol: SymbolRef) {
        self.captured.push(symbol);
    }

    pub fn captured(&self) -> Vec<SymbolRef> {
        self.captured.clone()
    }

    pub fn find(&self, name: &str) -> Option<SymbolRef> {
        self.bindings
            .iter()
            .find(|s| s.borrow().name == name)
            .cloned()
    }
}

#[derive(Clone)]
pub struct LocalContext {
    /** Map of existing symbols */
    pub symbols: Vec<SymbolHandle>,

    pub(crate) scopes: Vec<Scope>,

    pub(crate) type_store: TypeStore,
    pub expressions: HashMap<Location, TypeId>,

    pub current_declaration_dependencies: Option<Vec<SymbolRef>>,
    pub other_dependencies: HashMap<Location, Vec<SymbolRef>>,
}

impl LocalContext {
    pub fn new() -> Self {
        Self {
            symbols: Vec::<SymbolHandle>::new(),
            scopes: vec![Scope::new()],
            type_store: TypeStore::new(),
            expressions: HashMap::new(),
            current_declaration_dependencies: None,
            other_dependencies: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(Scope::new());
    }
    pub fn pop_scope(&mut self) -> Scope {
        if self.scopes.len() <= 1 {
            panic!("Cannot pop module scope");
        }
        self.scopes.pop().unwrap()
    }
    fn current_scope(&self) -> &Scope {
        self.scopes.last().unwrap()
    }
    fn current_scope_mut(&mut self) -> &mut Scope {
        self.scopes.last_mut().unwrap()
    }

    pub fn register_symbol(&mut self, symbol: SymbolData) -> SymbolRef {
        for dep in &symbol.dependencies {
            if !self.current_scope().has(dep) {
                self.current_scope_mut().capture(dep.clone());
            }
        }

        let handle = SymbolHandle::new(symbol);
        self.current_scope_mut().bindings.push(handle.readonly());
        let r = handle.readonly();
        self.symbols.push(handle);
        r
    }

    pub fn import(&mut self, symbol: SymbolRef) {
        let current_scope = self.current_scope_mut();
        current_scope.bindings.push(symbol.clone());
    }

    pub fn save_expression_type(&mut self, loc: Location, ty: TypeId) -> TypeId {
        self.expressions.insert(loc, ty);
        ty
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolRef> {
        for scope in self.scopes.iter().rev() {
            if let Some(var) = scope.find(name) {
                return Some(var);
            }
        }
        None
    }
    pub fn lookup_mut(&self, name: &str) -> Option<SymbolHandle> {
        match self.lookup(name) {
            Some(var) => self.get_handle(&var),
            None => None,
        }
    }
    fn get_handle(&self, var: &SymbolRef) -> Option<SymbolHandle> {
        self.symbols.iter().find(|s| s.has_ref(var)).cloned()
    }

    pub fn find_in_current_scope(&self, name: &str) -> Option<SymbolRef> {
        self.current_scope()
            .bindings
            .iter()
            .find(|b| b.borrow().name == *name)
            .cloned()
    }

    pub fn add_dependencies(&mut self, deps: Vec<SymbolRef>) {
        let Some(current_dependencies) = self.current_declaration_dependencies.as_mut() else {
            return;
        };
        current_dependencies.extend(deps);
    }
}
