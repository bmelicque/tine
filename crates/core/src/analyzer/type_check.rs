use std::{collections::HashMap, rc::Rc};

use pest::Span;
use swc_common::FileName;

use crate::{
    analyzer::graph::ParsedModule,
    ast::Program,
    type_checker::{self, dom_metadata, CheckData, CheckResult, TypeChecker},
    types::{Type, TypeId},
    ParseError, SymbolRef, Token, TypeStore,
};

#[derive(Debug, Clone)]
pub struct ModuleTypeData {
    pub type_store: Rc<TypeStore>,
    pub exports: Vec<SymbolRef>,
    pub expressions: HashMap<Span<'static>, u32>,
    pub tokens: HashMap<Span<'static>, Token>,
    pub dependencies: HashMap<Span<'static>, Vec<SymbolRef>>,
}

impl ModuleTypeData {
    pub fn resolve_type(&self, id: TypeId) -> &Type {
        self.type_store.get(id)
    }
}

#[derive(Debug, Clone)]
pub struct CheckedModule {
    pub name: Rc<FileName>,
    pub ast: Program,
    pub metadata: ModuleTypeData,
    pub errors: Vec<ParseError>,
}

impl CheckedModule {
    pub fn dummy() -> Self {
        Self {
            name: Rc::new(FileName::Custom("".into())),
            ast: Program::dummy(),
            metadata: ModuleTypeData {
                type_store: Rc::new(TypeStore::new()),
                exports: vec![],
                expressions: HashMap::new(),
                tokens: HashMap::new(),
                dependencies: HashMap::new(),
            },
            errors: vec![],
        }
    }
}

pub fn type_check(mut modules: Vec<ParsedModule>) -> Vec<CheckedModule> {
    let mut type_store = TypeStore::new();
    let mut check_data = HashMap::new();
    for module in &mut modules {
        let mut result = type_check_module(&check_data, module, type_store);
        module.errors.append(&mut result.errors);
        check_data.insert(module.name.clone(), result.data);
        type_store = result.type_store;
    }

    let type_store = Rc::new(type_store);
    modules
        .into_iter()
        .map(|module| {
            let check_data = check_data.remove(&module.name).unwrap();
            CheckedModule {
                name: module.name,
                ast: module.ast,
                metadata: ModuleTypeData {
                    type_store: type_store.clone(),
                    exports: check_data.exports,
                    expressions: check_data.expressions,
                    tokens: check_data.tokens,
                    dependencies: check_data.dependencies,
                },
                errors: module.errors,
            }
        })
        .collect()
}

fn type_check_module(
    checked_modules: &HashMap<Rc<FileName>, CheckData>,
    module: &ParsedModule,
    store: TypeStore,
) -> CheckResult {
    match *module.name {
        FileName::Real(_) => {
            let checker = TypeChecker::with_store(checked_modules.clone(), store);
            checker.check(&module)
        }
        FileName::Custom(ref name) => type_check_virtual_module(name, store),
        _ => unreachable!("unexpected FileName variant"),
    }
}

fn type_check_virtual_module(name: &str, store: TypeStore) -> type_checker::CheckResult {
    match name {
        "dom" => dom_metadata(store),
        _ => panic!("unexpected module name '{}'", name),
    }
}
