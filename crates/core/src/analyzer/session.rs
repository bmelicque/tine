use std::{
    collections::HashMap,
    sync::{Mutex, MutexGuard},
};

use anyhow::anyhow;

use crate::{
    analysis_context::{symbols::SymbolId, type_store::TypeStore},
    analyzer::{graph::ModuleGraph, loader::ModuleLoader, modules::Module, ModuleId},
    ast, ir, pretty_print_error,
    type_checker::analysis_context::{
        symbols::{MethodSymbol, SymbolTable},
        type_display::{display_raw_type, display_type},
    },
    types::{Type, TypeId},
    Diagnostic, ModulePath,
};

pub type SessionLoader = dyn ModuleLoader + Sync + Send;

pub struct Session {
    /// The entry point of the project. It should be a `ModulePath::Real`.
    pub(super) entry_point: ModulePath,
    pub(super) loader: Box<SessionLoader>,
    pub(super) module_graph: ModuleGraph,
    /// The parsed AST for each module.
    pub(super) parsed: HashMap<ModuleId, ast::Program>,
    pub(super) ir: HashMap<ModuleId, ir::Program>,
    /// The global type store, that can be read and written through each
    /// module's type checker.
    pub(super) types: Mutex<TypeStore>,
    /// An arena for all the symbols (i.e. names) declared and defined accross
    /// the project.
    pub symbols: SymbolTable,
    /// A list of builtin functions and types
    pub(super) builtins: Vec<SymbolId>,
    /// All symbols exported by each module.
    pub(super) exports: HashMap<ModuleId, Vec<SymbolId>>,
    pub(super) diagnostics: HashMap<ModuleId, Vec<Diagnostic>>,
}

impl Session {
    pub fn new(loader: Box<SessionLoader>) -> Self {
        let mut session = Self {
            entry_point: ModulePath::Virtual("".into()),
            loader,
            module_graph: ModuleGraph::new(),
            parsed: HashMap::new(),
            ir: HashMap::new(),
            types: Mutex::new(TypeStore::new()),
            symbols: SymbolTable::default(),
            builtins: Vec::new(),
            exports: HashMap::new(),
            diagnostics: HashMap::new(),
        };
        session.init_builtins();
        session
    }

    /// Analyze the whole project, starting from the entry_point.
    /// Returns the `ModuleId` of the entry point.
    pub fn analyze(&mut self, entry_point: ModulePath) -> anyhow::Result<ModuleId> {
        assert!(matches!(entry_point, ModulePath::Real(_)));
        self.entry_point = entry_point.clone();
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

    pub fn get_ast(&self, id: ModuleId) -> &ast::Program {
        &self.parsed.get(&id).unwrap()
    }
    pub fn get_ir(&self, id: ModuleId) -> &ir::Program {
        &self.ir.get(&id).unwrap()
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

    pub fn modules(&self) -> Vec<&Module> {
        self.module_graph.nodes.iter().collect()
    }

    pub fn types(&self) -> MutexGuard<'_, TypeStore> {
        self.types.lock().unwrap()
    }

    pub fn find_type(&self, ty: &Type) -> Option<TypeId> {
        self.types.lock().unwrap().find_id(ty)
    }
    pub fn display_type(&self, id: TypeId) -> String {
        let types = self.types.lock().unwrap();
        display_type(&types, id)
    }
    pub fn display_raw_type(&self, id: TypeId) -> String {
        let types = self.types.lock().unwrap();
        display_raw_type(&types, id)
    }

    pub fn find_method(&self, name: &str, ty: TypeId) -> Option<&MethodSymbol> {
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
}
