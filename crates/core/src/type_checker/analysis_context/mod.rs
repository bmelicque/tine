mod symbols;
pub mod type_store;

pub use symbols::{SymbolData, SymbolHandle, SymbolKind, SymbolRef};

use std::collections::HashMap;

use crate::{
    locations::Span, type_checker::analysis_context::type_store::TypeStore, types::TypeId,
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
    pub outer_id: Option<Span>,
    /**
     * Lists symbols captured by the scope
     */
    captured: Vec<SymbolRef>,
}

impl Scope {
    pub fn new(within: Option<Span>) -> Self {
        Self {
            bindings: vec![],
            outer_id: within,
            captured: vec![],
        }
    }

    pub fn captured(&self) -> Vec<SymbolRef> {
        self.captured.clone()
    }
}

#[derive(Clone)]
pub struct AnalysisContext {
    /** Map of existing symbols */
    pub symbols: Vec<SymbolHandle>,

    pub scopes: HashMap<Span, Scope>,
    current_scope: Option<Span>,

    pub(crate) type_store: TypeStore,
    pub expressions: HashMap<Span, TypeId>,
    pub tokens: HashMap<Span, Token>,

    pub current_declaration_dependencies: Option<Vec<SymbolRef>>,
    pub other_dependencies: HashMap<Span, Vec<SymbolRef>>,
}

impl AnalysisContext {
    pub fn new() -> Self {
        Self {
            symbols: Vec::<SymbolHandle>::new(),
            scopes: HashMap::new(),
            current_scope: None,
            type_store: TypeStore::new(),
            expressions: HashMap::new(),
            tokens: HashMap::new(),
            current_declaration_dependencies: None,
            other_dependencies: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self, span: Span) {
        self.scopes.insert(span, Scope::new(self.current_scope));
        self.current_scope = Some(span);
    }
    pub fn exit_scope(&mut self) -> &Scope {
        let current_scope = self.current_scope;
        let current = &self.scopes[&current_scope.expect("Cannot pop global scope")];
        self.current_scope = current.outer_id;
        current
    }
    fn get_scope(&self, id: Span) -> &Scope {
        self.scopes.iter().find(|(span, _)| **span == id).unwrap().1
    }
    fn get_current_scope(&self) -> &Scope {
        self.get_scope(self.current_scope.unwrap())
    }

    pub fn register_symbol(&mut self, symbol: SymbolData) -> SymbolRef {
        let mut scope = self.get_current_scope().clone();
        for dep in &symbol.dependencies {
            let is_captured = !dep
                .borrow()
                .defined_at
                .is_within(self.current_scope.unwrap());
            if is_captured {
                scope.captured.push(dep.clone());
            }
        }

        let handle = SymbolHandle::new(symbol);
        scope.bindings.push(handle.readonly());

        self.scopes.insert(self.current_scope.unwrap(), scope);
        self.symbols.push(handle.clone());
        handle.readonly()
    }

    pub fn save_expression_type(&mut self, span: Span, ty: TypeId) -> TypeId {
        self.expressions.insert(span, ty);
        ty
    }
    pub fn save_symbol_token(&mut self, span: Span, symbol: SymbolRef) {
        let token = Token::Symbol(SymbolToken { span, symbol });
        self.tokens.insert(span, token);
    }
    pub fn save_member_token(&mut self, span: Span, type_id: TypeId) {
        let token = Token::Member(MemberToken { span, ty: type_id });
        self.tokens.insert(span, token);
    }

    pub fn lookup(&self, name: &str) -> Option<SymbolRef> {
        let mut scope = self.get_current_scope();
        loop {
            let var = scope.bindings.iter().find(|b| b.borrow().name == *name);
            if var.is_some() {
                return var.cloned();
            }
            match scope.outer_id {
                Some(span) => scope = self.get_scope(span),
                None => return None,
            }
        }
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
        self.get_current_scope()
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

#[derive(Clone, Debug)]
pub struct CheckData {
    pub exports: Vec<SymbolRef>,
    pub expressions: HashMap<Span, TypeId>,
    pub tokens: HashMap<Span, Token>,
    pub dependencies: HashMap<Span, Vec<SymbolRef>>,
}
