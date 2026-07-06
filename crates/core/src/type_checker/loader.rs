use std::collections::HashMap;

use enum_from_derive::EnumFrom;

use crate::{
    ast,
    common::module_path::ModuleId,
    type_checker::std::{dom::check_dom_module, signals::check_signals_module},
    ModulePath, TypeChecker,
};

#[derive(EnumFrom)]
pub enum LoadedModule {
    Real(ast::Program),
    Virtual(Box<dyn Fn(&mut TypeChecker)>),
}

pub trait ModuleLoader {
    fn find_id(&self, name: &ModulePath) -> Option<ModuleId>;
    fn get_name(&self, module: ModuleId) -> &ModulePath;
    fn module(&mut self, id: ModuleId) -> LoadedModule;
}

pub struct CheckerLoader {
    pub names: Vec<ModulePath>,
    pub ids: HashMap<ModulePath, ModuleId>,
    pub ast: HashMap<ModuleId, ast::Program>,
}
impl ModuleLoader for CheckerLoader {
    fn find_id(&self, name: &ModulePath) -> Option<ModuleId> {
        self.ids.get(name).copied()
    }
    fn get_name(&self, module: ModuleId) -> &ModulePath {
        &self.names[module]
    }
    fn module(&mut self, id: ModuleId) -> LoadedModule {
        let name = self.get_name(id);
        match name {
            ModulePath::Virtual(v) => match v.as_str() {
                "dom" => {
                    let loader: Box<dyn Fn(&mut TypeChecker)> =
                        Box::new(move |tc| check_dom_module(id, tc));
                    return loader.into();
                }
                "signals" => {
                    let loader: Box<dyn Fn(&mut TypeChecker)> =
                        Box::new(move |tc| check_signals_module(id, tc));
                    return loader.into();
                }
                _ => {}
            },
            _ => {}
        }

        self.ast.remove(&id).unwrap().into()
    }
}

pub struct MockLoader;
impl ModuleLoader for MockLoader {
    fn find_id(&self, _name: &ModulePath) -> Option<ModuleId> {
        None
    }
    fn get_name(&self, _module: ModuleId) -> &ModulePath {
        panic!()
    }
    fn module(&mut self, _id: ModuleId) -> LoadedModule {
        ast::Program::dummy().into()
    }
}
