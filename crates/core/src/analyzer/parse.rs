use std::path::PathBuf;

use anyhow::bail;

use super::graph::ModuleGraph;

use crate::{
    analyzer::modules::{ModuleId, ModulePath, ParsedModule},
    common::use_decl_to_paths,
    parser::ParserEngine,
};

struct ProjectParser {
    graph: ModuleGraph,
}

impl ProjectParser {
    fn parse_entry(mut self, entry: &ModulePath) -> Result<ModuleGraph, anyhow::Error> {
        self.parse_file(entry)?;
        Ok(self.graph)
    }

    fn parse_file(&mut self, file_name: &ModulePath) -> Result<ModuleId, anyhow::Error> {
        let module = match file_name {
            ModulePath::Real(p) => parse_real_module(p)?,
            ModulePath::Virtual(c) => parse_virtual_module(c)?,
        };
        let file_names = get_dependencies(&module);

        let module_id = self.graph.add_module(module);

        for dependency_name in file_names {
            self.parse_dependency(&dependency_name, module_id);
        }

        Ok(module_id)
    }

    fn parse_dependency(&mut self, dependency_name: &ModulePath, parent_id: ModuleId) {
        if let Some(dependency_id) = self.graph.find_id(&dependency_name) {
            self.graph.add_edge(dependency_id, parent_id.clone());
            return;
        }
        let Ok(dependency) = self.parse_file(&dependency_name) else {
            todo!()
        };
        self.graph.add_edge(dependency, parent_id);
    }
}

fn parse_real_module(path: &PathBuf) -> anyhow::Result<ParsedModule> {
    let src = std::fs::read_to_string(path)?;
    let mut parser = ParserEngine::new();
    let result = parser.parse(&src);

    Ok(ParsedModule::builder()
        .name(path)
        .src(src.into())
        .ast(result.node)
        .errors(result.errors)
        .build())
}

fn parse_virtual_module(name: &String) -> anyhow::Result<ParsedModule> {
    match name.as_str() {
        "dom" => Ok(ParsedModule::builder().name("dom").build()),
        name => bail!("Cannot find module '{}'", name),
    }
}

fn get_dependencies(module: &ParsedModule) -> Vec<ModulePath> {
    let mut file_names: Vec<ModulePath> = module
        .ast
        .items
        .iter()
        .filter_map(|item| item.as_use_declaration_ref())
        .flat_map(|decl| use_decl_to_paths(&module.name, decl))
        .map(|imports| imports.module_name)
        .collect();
    file_names.sort();
    file_names.dedup();
    file_names
}

/// Parse a package starting from the given entry point, which should be the
/// program's main entry.
pub fn parse_package(entry: PathBuf) -> Result<ModuleGraph, anyhow::Error> {
    debug_assert!(std::path::Path::is_absolute(&entry));
    let filename = ModulePath::Real(std::fs::canonicalize(entry)?);
    let parser = ProjectParser {
        graph: ModuleGraph::new(),
    };
    parser.parse_entry(&filename)
}
