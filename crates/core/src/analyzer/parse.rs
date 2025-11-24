use std::{path::PathBuf, rc::Rc};

use anyhow::bail;
use swc_common::FileName;

use super::graph::{ModuleGraph, ParsedModule};

use crate::{ast, common::use_decl_to_paths, parser::ParserEngine};

struct ProjectParser {
    graph: ModuleGraph,
}

impl ProjectParser {
    fn parse_entry(mut self, entry: &FileName) -> Result<ModuleGraph, anyhow::Error> {
        self.parse_file(entry)?;
        Ok(self.graph)
    }

    fn parse_file(&mut self, file_name: &FileName) -> Result<&ParsedModule, anyhow::Error> {
        let module = match file_name {
            FileName::Real(p) => parse_file(p)?,
            FileName::Custom(c) => parse_virtual_module(c).unwrap(),
            _ => panic!("Unexpected file name"),
        };
        let file_names = get_dependencies(&module);

        let parsed_name = module.name.clone();
        self.graph.add_module(module);

        for dependency_name in file_names {
            self.parse_dependency(dependency_name, parsed_name.clone());
        }

        Ok(self.graph.get_module(&parsed_name).unwrap())
    }

    fn parse_dependency(&mut self, dependency_name: FileName, dependant_name: Rc<FileName>) {
        if let Some(dependency) = self.graph.get_module(&dependency_name) {
            self.graph
                .add_edge(dependency.name.clone(), dependant_name.clone());
            return;
        }
        let Ok(dependency) = self.parse_file(&dependency_name) else {
            todo!()
        };
        let dependency_name = dependency.name.clone();
        self.graph.add_edge(dependency_name, dependant_name.clone());
    }
}

fn parse_file(path: &PathBuf) -> Result<ParsedModule, anyhow::Error> {
    let src = std::fs::read_to_string(path)?;
    let src = Box::leak(src.into_boxed_str());
    let mut parser = ParserEngine::new();
    let result = parser.parse(src);

    Ok(ParsedModule {
        name: Rc::new(FileName::Real(path.clone())),
        ast: result.node,
        errors: result.errors,
    })
}

fn parse_virtual_module(name: &String) -> anyhow::Result<ParsedModule> {
    match name.as_str() {
        "dom" => Ok(ParsedModule {
            name: Rc::new(FileName::Custom(name.clone())),
            ast: ast::Program::dummy(),
            errors: Vec::new(),
        }),
        name => bail!("Cannot find module '{}'", name),
    }
}

fn get_dependencies(module: &ParsedModule) -> Vec<FileName> {
    let mut file_names: Vec<FileName> = module
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

/// Parse a package starting from the given entry point
pub fn parse_package(entry: PathBuf) -> Result<ModuleGraph, anyhow::Error> {
    let filename = FileName::Real(std::fs::canonicalize(entry)?);
    let parser = ProjectParser {
        graph: ModuleGraph::new(),
    };
    parser.parse_entry(&filename)
}
