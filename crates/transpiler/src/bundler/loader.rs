use crate::{
    bundler::internals::{parse_dom, parse_internals},
    codegen::CodeGenerator,
};
use mylang_core::Module;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use swc_common::{FileName, SourceMap};

pub struct SwcLoader {
    modules: Vec<Rc<RefCell<Module>>>,
}

impl SwcLoader {
    pub fn new(modules: Vec<Rc<RefCell<Module>>>) -> Self {
        Self { modules }
    }

    fn load_real_module(&self, file: &FileName) -> anyhow::Result<swc_bundler::ModuleData> {
        let Some(module) = self.modules.iter().find(|m| *m.borrow().name == *file) else {
            panic!("couldn't find module '{:?}'", file)
        };
        let module = module.borrow();

        let cm = Arc::new(SourceMap::default());
        let fm = cm.new_source_file(
            swc_common::sync::Lrc::new(file.clone()),
            module.ast.span.as_str(),
        );

        let mut code_generator =
            CodeGenerator::new(file.clone(), module.context.as_ref().unwrap().clone());
        let module = code_generator.program_to_swc_module(&module.ast);

        Ok(swc_bundler::ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }
}

impl swc_bundler::Load for SwcLoader {
    fn load(&self, file: &FileName) -> anyhow::Result<swc_bundler::ModuleData> {
        match file {
            FileName::Real(_) => self.load_real_module(file),
            FileName::Custom(name) => match name.as_str() {
                "dom" => Ok(parse_dom()),
                "internals" => Ok(parse_internals()),
                name => panic!("unexpected name '{}'", name),
            },
            _ => unreachable!("unexpected FileName variant"),
        }
    }
}
