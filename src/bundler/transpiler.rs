use std::{cell::RefCell, path::PathBuf, rc::Rc};

use swc_common::FileName;

use crate::{
    bundler::{bundle_entry, parse_package, utils::print_errors, Module, SwcLoader, SwcResolver},
    type_checker::{self, dom_context, CheckResult, TypeChecker},
    utils::pretty_print_error,
};

pub fn transpile(entry_point: PathBuf, out: &str) {
    let graph = match parse_package(entry_point.clone()) {
        Ok(graph) => graph,
        Err(e) => {
            println!("{:?}", e);
            return;
        }
    };

    let modules = match graph.try_sorted_vec() {
        Ok(modules) => modules,
        Err(edges) => {
            // TODO: add cycle errors in modules
            graph.use_errors(|e| pretty_print_error(&e));
            return;
        }
    };

    type_check(&modules);
    if print_errors(&modules) {
        return;
    }

    let resolver = SwcResolver::new();
    let loader = SwcLoader::new(graph.read_nodes());
    let _ = bundle_entry(entry_point, out, loader, resolver);
}

fn type_check(modules: &Vec<Rc<RefCell<Module>>>) {
    for module in modules {
        if module.borrow().context.is_some() {
            continue;
        }
        let mut result = type_check_module(&modules, &module.borrow());
        let mut module_mut = module.borrow_mut();
        module_mut.errors.append(&mut result.errors);
        module_mut.context = Some(result.metadata);
    }
}

fn type_check_module(
    checked_modules: &Vec<Rc<RefCell<Module>>>,
    module: &Module,
) -> type_checker::CheckResult {
    match *module.name {
        FileName::Real(_) => {
            let checker = TypeChecker::new(checked_modules.clone());
            checker.check(&module.ast)
        }
        FileName::Custom(ref name) => type_check_virtual_module(name),
        _ => unreachable!("unexpected FileName variant"),
    }
}

fn type_check_virtual_module(name: &str) -> type_checker::CheckResult {
    match name {
        "dom" => CheckResult {
            metadata: dom_context(),
            errors: Vec::new(),
        },
        _ => panic!("unexpected module name '{}'", name),
    }
}
