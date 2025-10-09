use std::{cell::RefCell, path::PathBuf, rc::Rc};

use anyhow::bail;
use swc_common::FileName;

use crate::{
    ast,
    bundler::{
        graph::{Module, ModuleGraph},
        utils::use_decl_to_paths,
    },
    parser::ParserEngine,
};

struct ProjectParser {
    graph: ModuleGraph,
}

impl ProjectParser {
    fn parse_entry(mut self, entry: &FileName) -> Result<ModuleGraph, anyhow::Error> {
        self.parse_file(entry)?;
        Ok(self.graph)
    }

    fn parse_file(&mut self, file_name: &FileName) -> Result<Rc<RefCell<Module>>, anyhow::Error> {
        if let Some(module) = self.graph.get_module(file_name) {
            return Ok(module);
        }

        let module = match file_name {
            FileName::Real(p) => parse_file(p)?,
            FileName::Custom(c) => parse_virtual_module(c).unwrap(),
            _ => panic!("Unexpected file name"),
        };
        let file_names = get_dependencies(&module);

        let name = module.name.clone();
        for file_name in file_names {
            match self.parse_file(&file_name) {
                Ok(child) => self.graph.add_edge(&module, &child.borrow()),
                Err(_) => {
                    todo!()
                }
            }
        }
        self.graph.add_module(module);

        Ok(self.graph.get_module(&name).unwrap())
    }
}

fn parse_file(path: &PathBuf) -> Result<Module, anyhow::Error> {
    let src = std::fs::read_to_string(path)?;
    let src = Box::leak(src.into_boxed_str());
    let mut parser = ParserEngine::new();
    let result = parser.parse(src);

    Ok(Module {
        name: Rc::new(FileName::Real(path.clone())),
        ast: result.node,
        context: None,
        errors: result.errors,
    })
}

fn parse_virtual_module(name: &String) -> anyhow::Result<Module> {
    match name.as_str() {
        "dom" => Ok(Module {
            name: Rc::new(FileName::Custom(name.clone())),
            ast: ast::Program::dummy(),
            context: None,
            errors: Vec::new(),
        }),
        name => bail!("Cannot find module '{}'", name),
    }
}

fn get_dependencies(module: &Module) -> Vec<FileName> {
    let mut file_names: Vec<FileName> = module
        .ast
        .items
        .iter()
        .filter_map(|item| item.as_use_declaration_ref())
        .flat_map(|decl| use_decl_to_paths(&module.name, decl))
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
