use crate::{
    bundler::internals::{parse_dom, parse_internals},
    codegen::CodeGenerator,
};
use mylang_core::{CheckedModule, ModulePath};
use std::sync::Arc;
use swc_common::{FileName, SourceMap};

pub struct SwcLoader {
    modules: Vec<CheckedModule>,
}

impl SwcLoader {
    pub fn new(modules: Vec<CheckedModule>) -> Self {
        Self { modules }
    }

    // TODO: avoid all this cloning
    fn load_real_module(&self, file: &FileName) -> anyhow::Result<swc_bundler::ModuleData> {
        let module = self.modules.iter().find(|m| match (&m.name, file) {
            (ModulePath::Real(a), FileName::Real(b)) => a == b,
            (ModulePath::Virtual(a), FileName::Custom(b)) => a == b,
            _ => false,
        });
        let Some(module) = module else {
            panic!("couldn't find module '{:?}'", file)
        };

        let cm = Arc::new(SourceMap::default());
        let fm = cm.new_source_file(
            swc_common::sync::Lrc::new(file.clone()),
            module.src.text().to_string(),
        );

        let mut code_generator = CodeGenerator::new(file.clone(), module.clone());
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
