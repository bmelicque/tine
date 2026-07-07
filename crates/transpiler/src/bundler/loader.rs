use crate::{
    bundler::internals::{parse_dom, parse_internals, parse_signals},
    codegen::CodeGenerator,
};
use std::{collections::HashMap, sync::Arc};
use swc_common::{FileName, SourceMap};
use tine_core::{ir, symbols::SymbolTable, type_store::TypeStore, ModuleId, ModulePath, Source};

pub struct SwcLoader {
    pub sources: HashMap<ModuleId, Source>,
    pub ir: HashMap<ModuleId, ir::Program>,
    pub ids: HashMap<ModulePath, ModuleId>,
    pub types: TypeStore,
    pub symbols: SymbolTable,
}

impl SwcLoader {
    // TODO: avoid all this cloning
    fn load_real_module(&self, file: &FileName) -> anyhow::Result<swc_bundler::ModuleData> {
        let module_path = match file {
            FileName::Real(f) => ModulePath::Real(f.canonicalize()?.clone()),
            FileName::Custom(f) => ModulePath::Virtual(f.clone()),
            _ => unreachable!(),
        };
        let Some(module_id) = self.ids.get(&module_path).copied() else {
            panic!("couldn't find module '{:?}'", file)
        };

        let module = self.ir.get(&module_id).unwrap();
        let cm = Arc::new(SourceMap::default());
        let fm = cm.new_source_file(
            swc_common::sync::Lrc::new(file.clone()),
            self.sources
                .get(&module_id)
                .as_ref()
                .unwrap()
                .text()
                .to_string(),
        );

        let mut code_generator = CodeGenerator::new(module_path, &self.types, &self.symbols);
        let module = code_generator.program_to_swc_module(module.clone());

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
                "$internals" => Ok(parse_internals()),
                "signals" => Ok(parse_signals()),
                name => panic!("unexpected name '{}'", name),
            },
            _ => unreachable!("unexpected FileName variant"),
        }
    }
}
