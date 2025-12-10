use std::path::PathBuf;

use anyhow::anyhow;

use crate::utils::pretty_print_error;

mod graph;
mod modules;
mod parse;
mod type_check;

pub use modules::{ModuleId, ModulePath, ParsedModule};
pub use type_check::{CheckedModule, ModuleTypeData};

pub struct AnalyzedModules {
    pub modules: Vec<CheckedModule>,
    /// Index of the main entry in the list of checked modules
    pub entry: usize,
}

pub fn analyze(entry_point: PathBuf) -> Result<AnalyzedModules, anyhow::Error> {
    let graph = match parse::parse_package(entry_point.clone()) {
        Ok(graph) => graph,
        Err(e) => {
            eprintln!("{:?}", e);
            return Err(e);
        }
    };

    let sort_result = graph.try_sorted_vec();
    if sort_result.unsorted.len() > 0 {
        graph.errors().for_each(|e| pretty_print_error(&e));
        return Err(anyhow!("cannot resolve module graph"));
    }
    let modules = graph.into_ordered_nodes(&sort_result.sorted);

    let modules = type_check::type_check(modules);
    let filename = ModulePath::Real(entry_point.canonicalize().unwrap());
    let entry = modules.iter().position(|m| m.name == filename).unwrap();
    Ok(AnalyzedModules { modules, entry })
}
