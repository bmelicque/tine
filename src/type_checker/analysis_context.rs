use std::collections::{HashMap, HashSet};

use pest::Span;

use crate::{type_checker::utils::within, types};

pub type SymbolId = usize;

pub struct Symbol {
    pub name: String,
    pub ty: types::Type,
    pub defined_at: Span<'static>,
    pub mutable: bool,
    pub reads: usize,
    pub writes: usize,
    pub mut_refs: usize,
    pub ro_refs: usize,
    pub dependencies: Vec<SymbolId>,
}

impl Symbol {
    pub fn new(
        name: String,
        ty: types::Type,
        mutable: bool,
        defined_at: Span<'static>,
        dependencies: Vec<SymbolId>,
    ) -> Self {
        Self {
            name,
            ty,
            defined_at,
            mutable,
            reads: 0,
            writes: 0,
            mut_refs: 0,
            ro_refs: 0,
            dependencies,
        }
    }

    pub fn pure(name: String, ty: types::Type, defined_at: Span<'static>) -> Self {
        Self {
            name,
            ty,
            defined_at,
            mutable: false,
            reads: 0,
            writes: 0,
            mut_refs: 0,
            ro_refs: 0,
            dependencies: vec![],
        }
    }

    pub fn has_ref(&self) -> bool {
        self.mut_refs + self.ro_refs > 0
    }

    pub fn can_be_reactive(&self) -> bool {
        self.mutable || self.dependencies.len() > 0
    }
}

#[derive(Clone)]
pub struct Scope {
    pub bindings: Vec<SymbolId>,
    pub outer_id: Option<Span<'static>>,
    /**
     * Lists symbols captured by the scope
     */
    captured: HashSet<SymbolId>,
}

impl Scope {
    pub fn new(within: Option<Span<'static>>) -> Self {
        Self {
            bindings: vec![],
            outer_id: within,
            captured: HashSet::new(),
        }
    }

    pub fn captured(&self) -> HashSet<SymbolId> {
        self.captured.clone()
    }
}

pub struct AnalysisContext {
    /** Map of existing symbols */
    pub symbols: Vec<Symbol>,

    /** Map of Identifier nodes to their symbol's id */
    pub bindings: HashMap<Span<'static>, SymbolId>,

    pub scopes: HashMap<Span<'static>, Scope>,
    current_scope: Option<Span<'static>>,

    pub types: HashMap<Span<'static>, types::Type>,

    pub current_declaration_dependencies: Option<Vec<SymbolId>>,
    pub other_dependencies: HashMap<Span<'static>, Vec<SymbolId>>,
}

impl AnalysisContext {
    pub fn new() -> Self {
        Self {
            symbols: Vec::<Symbol>::new(),
            bindings: HashMap::new(),
            scopes: HashMap::new(),
            current_scope: None,
            types: HashMap::new(),
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

    pub fn register_symbol(&mut self, symbol: Symbol) -> SymbolId {
        let id: SymbolId = self.symbols.len();
        let mut scope = self.get_current_scope().clone();
        scope.bindings.push(id);
        for dep_id in &symbol.dependencies {
            let dep = &self.symbols[*dep_id];
            if !within(self.current_scope.unwrap(), dep.defined_at) {
                scope.captured.insert(*dep_id);
            }
        }
        self.scopes.insert(self.current_scope.unwrap(), scope);
        self.symbols.push(symbol);
        id
    }

    pub fn get_id(&self, name: &str) -> Option<SymbolId> {
        let mut scope = self.get_current_scope();
        loop {
            let id = scope
                .bindings
                .iter()
                .find(|id| self.symbols[**id].name == name);
            if id.is_some() {
                return id.copied();
            }
            match scope.outer_id {
                Some(span) => scope = self.get_scope(span),
                None => return None,
            }
        }
    }
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.get_id(name).map(|id| self.symbols.get(id)).flatten()
    }
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.get_id(name)
            .map(|id| self.symbols.get_mut(id))
            .flatten()
    }

    pub fn add_dependencies(&mut self, deps: Vec<SymbolId>) {
        let Some(current_dependencies) = self.current_declaration_dependencies.as_mut() else {
            return;
        };
        current_dependencies.extend(deps);
    }
}
