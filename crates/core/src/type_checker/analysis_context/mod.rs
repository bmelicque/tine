pub mod type_store;
mod variables;

pub use variables::{VariableData, VariableHandle, VariableRef};

use std::{collections::HashMap, rc::Rc};

use pest::Span;

use crate::{
    type_checker::analysis_context::type_store::TypeStore,
    types::{self, Type, TypeId},
};

#[derive(Clone, Debug)]
pub enum Token {
    Symbol(SymbolToken),
    Member(MemberToken),
}

#[derive(Clone, Debug)]
pub struct SymbolToken {
    pub span: Span<'static>,
    pub symbol: VariableRef,
}

#[derive(Clone, Debug)]
pub struct MemberToken {
    pub span: Span<'static>,
    pub ty: Rc<types::Type>,
}

#[derive(Clone)]
pub struct Scope {
    pub bindings: Vec<VariableRef>,
    pub outer_id: Option<Span<'static>>,
    /**
     * Lists symbols captured by the scope
     */
    captured: Vec<VariableRef>,
}

impl Scope {
    pub fn new(within: Option<Span<'static>>) -> Self {
        Self {
            bindings: vec![],
            outer_id: within,
            captured: vec![],
        }
    }

    pub fn captured(&self) -> Vec<VariableRef> {
        self.captured.clone()
    }
}

#[derive(Clone)]
pub struct AnalysisContext {
    /** Map of existing symbols */
    pub symbols: Vec<VariableHandle>,

    pub scopes: HashMap<Span<'static>, Scope>,
    current_scope: Option<Span<'static>>,

    /// FIXME:
    /// This property is legacy code and will disappear
    pub types: HashMap<Span<'static>, types::Type>,
    pub(crate) type_store: TypeStore,
    pub expressions: HashMap<Span<'static>, TypeId>,
    pub tokens: HashMap<Span<'static>, Token>,

    pub current_declaration_dependencies: Option<Vec<VariableRef>>,
    pub other_dependencies: HashMap<Span<'static>, Vec<VariableRef>>,
}

impl AnalysisContext {
    pub fn new() -> Self {
        Self {
            symbols: Vec::<VariableHandle>::new(),
            scopes: HashMap::new(),
            current_scope: None,
            types: HashMap::new(),
            type_store: TypeStore::new(),
            expressions: HashMap::new(),
            tokens: HashMap::new(),
            current_declaration_dependencies: None,
            other_dependencies: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self, span: Span<'static>) {
        self.scopes.insert(span, Scope::new(self.current_scope));
        self.current_scope = Some(span);
    }
    pub fn exit_scope(&mut self) -> &Scope {
        let current_scope = self.current_scope;
        let current = &self.scopes[&current_scope.expect("Cannot pop global scope")];
        self.current_scope = current.outer_id;
        current
    }
    fn get_scope(&self, id: Span<'static>) -> &Scope {
        self.scopes.iter().find(|(span, _)| **span == id).unwrap().1
    }
    fn get_current_scope(&self) -> &Scope {
        self.get_scope(self.current_scope.unwrap())
    }

    pub fn register_symbol(&mut self, symbol: VariableData) -> VariableRef {
        let mut scope = self.get_current_scope().clone();
        for dep in &symbol.dependencies {
            if !within(self.current_scope.unwrap(), dep.borrow().defined_at) {
                scope.captured.push(dep.clone());
            }
        }

        let handle = VariableHandle::new(symbol);
        scope.bindings.push(handle.readonly());

        self.scopes.insert(self.current_scope.unwrap(), scope);
        self.symbols.push(handle.clone());
        handle.readonly()
    }

    pub fn save_expression_type(&mut self, span: Span<'static>, ty: TypeId) -> TypeId {
        self.expressions.insert(span, ty);
        ty
    }
    pub fn save_symbol_token(&mut self, span: Span<'static>, symbol: VariableRef) {
        let token = Token::Symbol(SymbolToken { span, symbol });
        self.tokens.insert(span, token);
    }

    pub fn lookup(&self, name: &str) -> Option<VariableRef> {
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
    pub fn lookup_mut(&self, name: &str) -> Option<VariableHandle> {
        match self.lookup(name) {
            Some(var) => self.get_handle(&var),
            None => None,
        }
    }
    fn get_handle(&self, var: &VariableRef) -> Option<VariableHandle> {
        self.symbols.iter().find(|s| s.has_ref(var)).cloned()
    }

    pub fn find_in_current_scope(&self, name: &str) -> Option<VariableRef> {
        self.get_current_scope()
            .bindings
            .iter()
            .find(|b| b.borrow().name == *name)
            .cloned()
    }

    pub fn add_dependencies(&mut self, deps: Vec<VariableRef>) {
        let Some(current_dependencies) = self.current_declaration_dependencies.as_mut() else {
            return;
        };
        current_dependencies.extend(deps);
    }
}

fn within(outer: Span<'static>, inner: Span<'static>) -> bool {
    inner.start() >= outer.start() && inner.end() <= outer.end()
}

#[derive(Clone, Debug)]
pub struct ModuleMetadata {
    type_store: TypeStore,
    pub exports: Vec<VariableRef>,
    pub types: HashMap<Span<'static>, types::Type>,
    pub dependencies: HashMap<Span<'static>, Vec<VariableRef>>,
}

impl ModuleMetadata {
    pub fn new() -> Self {
        ModuleMetadata {
            type_store: TypeStore::new(),
            exports: vec![],
            types: HashMap::new(),
            dependencies: HashMap::new(),
        }
    }

    pub fn lookup(&self, name: &str) -> Option<VariableRef> {
        self.exports
            .iter()
            .find(|e| e.borrow().name == *name)
            .cloned()
    }

    pub fn resolve_type(&self, type_id: TypeId) -> Type {
        self.type_store.get(type_id).clone()
    }
}

impl From<&AnalysisContext> for ModuleMetadata {
    fn from(value: &AnalysisContext) -> Self {
        let main_scope = value
            .scopes
            .values()
            .find(|s| s.outer_id.is_none())
            .unwrap();
        let exports = main_scope.bindings.clone();

        Self {
            type_store: value.type_store.clone(),
            exports,
            types: value.types.clone(),
            dependencies: value.other_dependencies.clone(),
        }
    }
}
