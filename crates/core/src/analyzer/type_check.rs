use std::{collections::HashMap, rc::Rc};

use crate::{
    analyzer::{session::Session, ModuleId, ModulePath},
    locations::Span,
    type_checker::{CheckResult, TypeChecker},
    types::{Type, TypeId},
    SymbolRef, Token, TypeStore,
};

#[derive(Debug, Clone)]
pub struct ModuleTypeData {
    pub type_store: Rc<TypeStore>,
    pub exports: Vec<SymbolRef>,
    pub expressions: HashMap<Span, u32>,
    pub tokens: HashMap<Span, Token>,
    pub dependencies: HashMap<Span, Vec<SymbolRef>>,
}

impl ModuleTypeData {
    pub fn resolve_type(&self, id: TypeId) -> &Type {
        self.type_store.get(id)
    }
}

impl Session {
    pub fn check_project(&mut self, sorted_modules: &[ModuleId]) {
        for &module_id in sorted_modules {
            let mut result = self.check_module(module_id);
            self.diagnostics
                .get_mut(&module_id)
                .unwrap()
                .append(&mut result.diagnostics);
            self.symbols.append(&mut result.symbols);
            self.exports.insert(module_id, result.exports);
            self.add_expressions(result.expressions);
            self.add_dependencies(result.dependencies);
        }
    }

    fn check_module(&mut self, id: ModuleId) -> CheckResult {
        let module = &self.module_graph.nodes[id];
        match module.name.clone() {
            ModulePath::Real(_) => {
                let checker = TypeChecker::new(&self, id);
                checker.check()
            }
            ModulePath::Virtual(name) => self.check_virtual_module(id, &name),
        }
    }

    fn check_virtual_module(&mut self, id: ModuleId, name: &str) -> CheckResult {
        match name {
            "dom" => self.check_dom_module(id),
            "signals" => self.check_signals_module(id),
            _ => panic!("unexpected module name '{}'", name),
        }
    }
}
