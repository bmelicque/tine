use enum_from_derive::EnumFrom;

use crate::{
    ast,
    common::module_path::ModuleId,
    type_checker::std::{dom::check_dom_module, signals::check_signals_module},
    ModulePath, ProjectParser, TypeChecker,
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

pub struct CheckerLoader(ProjectParser);
impl From<ProjectParser> for CheckerLoader {
    fn from(value: ProjectParser) -> Self {
        Self(value)
    }
}
impl ModuleLoader for CheckerLoader {
    fn find_id(&self, name: &ModulePath) -> Option<ModuleId> {
        self.0.get_id(name)
    }
    fn get_name(&self, module: ModuleId) -> &ModulePath {
        self.0.name(module)
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

        self.0.consume_ast(id).into()
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
