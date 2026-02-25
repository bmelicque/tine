use crate::{
    bundler::internals::{parse_dom, parse_internals, parse_signals},
    codegen::CodeGenerator,
};
use std::sync::Arc;
use swc_common::{FileName, SourceMap};
use tine_core::{ModulePath, Session};

pub struct SwcLoader<'sess> {
    session: &'sess Session,
}

impl SwcLoader<'_> {
    pub fn new<'sess>(session: &'sess Session) -> SwcLoader<'sess> {
        SwcLoader { session }
    }

    // TODO: avoid all this cloning
    fn load_real_module(&self, file: &FileName) -> anyhow::Result<swc_bundler::ModuleData> {
        let module_id = self
            .session
            .modules()
            .iter()
            .position(|m| match (&m.name, file) {
                (ModulePath::Real(a), FileName::Real(b)) => a == b,
                (ModulePath::Virtual(a), FileName::Custom(b)) => a == b,
                _ => false,
            });
        let Some(module_id) = module_id else {
            panic!("couldn't find module '{:?}'", file)
        };

        let module = self.session.read_module(module_id);
        let cm = Arc::new(SourceMap::default());
        let fm = cm.new_source_file(
            swc_common::sync::Lrc::new(file.clone()),
            module.src.text().to_string(),
        );

        let mut code_generator = CodeGenerator::new(self.session, module_id);
        let module = code_generator.program_to_swc_module();

        Ok(swc_bundler::ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }
}

impl swc_bundler::Load for SwcLoader<'_> {
    fn load(&self, file: &FileName) -> anyhow::Result<swc_bundler::ModuleData> {
        match file {
            FileName::Real(_) => self.load_real_module(file),
            FileName::Custom(name) => match name.as_str() {
                "dom" => Ok(parse_dom()),
                "internals" => Ok(parse_internals()),
                "signals" => Ok(parse_signals()),
                name => panic!("unexpected name '{}'", name),
            },
            _ => unreachable!("unexpected FileName variant"),
        }
    }
}
