use std::{cell::RefCell, rc::Rc};

use swc_common::FileName;

use crate::{
    analyzer::graph::Module,
    type_checker::{self, dom_metadata, CheckResult, TypeChecker},
};

pub fn type_check(modules: &Vec<Rc<RefCell<Module>>>) {
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

fn type_check_module(checked_modules: &Vec<Rc<RefCell<Module>>>, module: &Module) -> CheckResult {
    match *module.name {
        FileName::Real(_) => {
            let checker = TypeChecker::new(checked_modules.clone());
            checker.check(&module)
        }
        FileName::Custom(ref name) => type_check_virtual_module(name),
        _ => unreachable!("unexpected FileName variant"),
    }
}

fn type_check_virtual_module(name: &str) -> type_checker::CheckResult {
    match name {
        "dom" => CheckResult {
            metadata: dom_metadata(),
            errors: Vec::new(),
        },
        _ => panic!("unexpected module name '{}'", name),
    }
}
