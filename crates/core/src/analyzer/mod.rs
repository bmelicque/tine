use std::path::PathBuf;

use anyhow::anyhow;
use swc_common::FileName;

use crate::utils::pretty_print_error;

mod graph;
mod parse;
mod type_check;

pub use graph::ParsedModule;
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

    let modules = match graph.try_sorted_vec() {
        Ok(modules) => modules,
        Err(edges) => {
            // TODO: add cycle errors in modules
            graph.use_errors(|e| pretty_print_error(&e));
            return Err(anyhow!("cannot resolve module graph"));
        }
    };

    let modules = type_check::type_check(modules);
    let filename = FileName::Real(entry_point.canonicalize().unwrap());
    let entry = modules.iter().position(|m| *m.name == filename).unwrap();
    Ok(AnalyzedModules { modules, entry })
}
