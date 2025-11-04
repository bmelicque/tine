use std::{cell::RefCell, path::PathBuf, rc::Rc};

use anyhow::anyhow;
use swc_common::FileName;

use crate::utils::pretty_print_error;

mod graph;
mod parse;
mod type_check;

pub use graph::Module;

pub struct AnalyzedModules {
    pub modules: Vec<Rc<RefCell<Module>>>,
    pub entry: Rc<RefCell<Module>>,
}

pub fn analyze(entry_point: PathBuf) -> Result<AnalyzedModules, anyhow::Error> {
    let graph = match parse::parse_package(entry_point.clone()) {
        Ok(graph) => graph,
        Err(e) => {
            println!("{:?}", e);
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

    type_check::type_check(&modules);
    let filename = FileName::Real(entry_point.canonicalize().unwrap());
    let module = graph.get_module(&filename).unwrap();
    Ok(AnalyzedModules {
        modules,
        entry: module,
    })
}
