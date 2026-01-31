use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Mutex, MutexGuard},
};

use anyhow::anyhow;

use crate::{
    analyzer::{graph::ModuleGraph, modules::Module, ModuleId},
    ast::Program,
    pretty_print_error,
    type_checker::SymbolHandle,
    types::{Type, TypeId},
    Diagnostic, Location, ModulePath, SymbolKind, SymbolRef, TypeStore,
};

pub struct Session {
    /// The entry point of the project. It should be a `ModulePath::Real`.
    pub(super) entry_point: ModulePath,
    pub(super) module_graph: ModuleGraph,
    /// The parsed AST for each module.
    pub(super) parsed: HashMap<ModuleId, Program>,
    /// The global type store, that can be read and written through each
    /// module's type checker.
    pub(super) types: Mutex<TypeStore>,
    /// An arena for all the symbols (i.e. names) declared and defined accross
    /// the project. See `SymbolHandle` for more details.
    pub(super) symbols: Vec<SymbolHandle>,
    /// A list of builtin functions and types
    pub(super) builtins: Vec<SymbolRef>,
    /// All symbols exported by each module.
    pub(super) exports: HashMap<ModuleId, Vec<SymbolRef>>,
    /// The type of each relevant expression, with expressions identified by
    /// their `Location`.
    pub(super) expressions: HashMap<Location, TypeId>,
    /// Dependencies captured by some expressions (like reactive listeners or
    /// closures). Said expressions are identified by their location.
    pub(super) dependencies: HashMap<Location, Vec<SymbolRef>>,
    pub(super) diagnostics: HashMap<ModuleId, Vec<Diagnostic>>,
}

impl Session {
    pub fn new() -> Self {
        let mut session = Self {
            entry_point: ModulePath::Virtual("".into()),
            module_graph: ModuleGraph::new(),
            parsed: HashMap::new(),
            types: Mutex::new(TypeStore::new()),
            symbols: Vec::new(),
            builtins: Vec::new(),
            exports: HashMap::new(),
            expressions: HashMap::new(),
            dependencies: HashMap::new(),
            diagnostics: HashMap::new(),
        };
        session.init_builtins();
        session
    }

    /// Analyze the whole project, starting from the entry_point.
    /// Returns the `ModuleId` of the entry point.
    pub fn analyze(&mut self, entry_point: PathBuf) -> anyhow::Result<ModuleId> {
        let filename = ModulePath::Real(entry_point.canonicalize().unwrap());
        self.entry_point = filename;
        self.parse_project(entry_point)?;

        let sort_result = self.module_graph.try_sorted_vec();
        if sort_result.unsorted.len() > 0 {
            return Err(self.handle_cycle_dependencies());
        }

        self.check_project(&sort_result.sorted);
        let entry = self.module_graph.find_id(&self.entry_point).unwrap();
        Ok(entry)
    }

    fn handle_cycle_dependencies(&self) -> anyhow::Error {
        for (id, module) in self.module_graph.nodes.iter().enumerate() {
            let Some(errors) = self.diagnostics.get(&id) else {
                continue;
            };
            let src = &module.src;
            for error in errors {
                pretty_print_error(src, &error);
            }
        }
        anyhow!("cannot resolve module graph")
    }

    pub fn get_module_id(&self, name: ModulePath) -> Option<ModuleId> {
        self.module_graph.find_id(&name)
    }

    pub fn read_module(&self, id: ModuleId) -> &Module {
        &self.module_graph.nodes[id]
    }

    pub fn get_ast(&self, id: ModuleId) -> &Program {
        &self.parsed.get(&id).unwrap()
    }

    pub fn find_export(&self, module: ModuleId, name: &str) -> Option<SymbolRef> {
        self.exports
            .get(&module)
            .unwrap()
            .iter()
            .find(|e| e.borrow().name == name)
            .cloned()
    }

    pub fn intern(&self, ty: Type) -> TypeId {
        self.types.lock().unwrap().add(ty)
    }
    pub fn intern_unique(&self, ty: Type) -> TypeId {
        self.types.lock().unwrap().add_unique(ty)
    }

    pub fn get_type(&self, id: TypeId) -> Type {
        self.types.lock().unwrap().get(id).clone()
    }

    pub(super) fn add_expressions(&mut self, exprs: HashMap<Location, TypeId>) {
        self.expressions.reserve(exprs.len());
        for (loc, ty) in exprs {
            self.expressions.insert(loc, ty);
        }
    }

    pub(super) fn add_dependencies(&mut self, deps: HashMap<Location, Vec<SymbolRef>>) {
        self.dependencies.reserve(deps.len());
        for (loc, deps) in deps {
            self.dependencies.insert(loc, deps);
        }
    }

    pub fn modules(&self) -> Vec<&Module> {
        self.module_graph.nodes.iter().collect()
    }

    pub fn types(&self) -> MutexGuard<'_, TypeStore> {
        self.types.lock().unwrap()
    }

    pub fn symbols(&self) -> Vec<SymbolRef> {
        self.symbols.iter().map(|s| s.readonly()).collect()
    }

    pub fn get_handle(&self, symbol: SymbolRef) -> Option<SymbolHandle> {
        self.symbols.iter().find(|s| s.has_ref(&symbol)).cloned()
    }

    pub fn get_type_at(&self, loc: Location) -> Option<Type> {
        self.expressions.get(&loc).map(|t| self.get_type(*t))
    }
    pub fn find_type(&self, ty: &Type) -> Option<TypeId> {
        self.types.lock().unwrap().find_id(ty)
    }
    pub fn display_type(&self, id: TypeId) -> String {
        self.types.lock().unwrap().display_type(id)
    }
    pub fn display_raw_type(&self, id: TypeId) -> String {
        self.types.lock().unwrap().display_raw_type(id)
    }

    pub fn find_method(&self, name: &str, ty: TypeId) -> Option<SymbolRef> {
        self.symbols
            .iter()
            .find(|s| {
                let s = s.borrow();
                s.name == name
                    && match s.kind {
                        SymbolKind::Method { ref owner, .. } => owner.borrow().ty == ty,
                        _ => false,
                    }
            })
            .cloned()
            .map(|h| h.readonly())
    }

    pub fn diagnostics(&self) -> &HashMap<ModuleId, Vec<Diagnostic>> {
        &self.diagnostics
    }

    pub fn get_dependencies(&self, loc: Location) -> Option<&Vec<SymbolRef>> {
        self.dependencies.get(&loc)
    }
}
