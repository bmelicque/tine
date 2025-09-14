use std::{collections::HashMap, hash::Hash};

use pest::Span;

use crate::types;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SymbolId(u32);

pub struct Symbol {
    pub name: String,
    pub ty: types::Type,
    pub defined_at: Span<'static>,
    pub mutable: bool,
    pub reads: usize,
    pub writes: usize,
    pub mut_refs: usize,
    pub ro_refs: usize,
}

impl Symbol {
    pub fn new(name: String, ty: types::Type, mutable: bool, defined_at: Span<'static>) -> Self {
        Self {
            name,
            ty,
            defined_at,
            mutable,
            reads: 0,
            writes: 0,
            mut_refs: 0,
            ro_refs: 0,
        }
    }

    pub fn has_ref(&self) -> bool {
        self.mut_refs + self.ro_refs > 0
    }
}

pub struct AnalysisContext {
    /** Map of existing symbols */
    pub symbols: HashMap<SymbolId, Symbol>,

    /** Map of Identifier nodes to their symbol's id */
    pub bindings: HashMap<Span<'static>, SymbolId>,

    pub scopes: Vec<HashMap<String, SymbolId>>,

    pub types: HashMap<Span<'static>, types::Type>,
}

impl AnalysisContext {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::new(),
            bindings: HashMap::new(),
            scopes: vec![],
            types: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }
    pub fn exit_scope(&mut self) {
        self.scopes.pop().expect("Cannot pop the global scope");
    }

    pub fn register_symbol(&mut self, symbol: Symbol) -> SymbolId {
        let id = SymbolId(self.symbols.len() as u32);
        self.scopes
            .last_mut()
            .expect("No scope available")
            .insert(symbol.name.clone(), id.clone());
        self.symbols.insert(id.clone(), symbol);
        id
    }

    pub fn get_id(&self, name: &str) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(id.clone());
            }
        }
        None
    }
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.get_id(name).map(|id| self.symbols.get(&id)).flatten()
    }
    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        self.get_id(name)
            .map(|id| self.symbols.get_mut(&id))
            .flatten()
    }
}
