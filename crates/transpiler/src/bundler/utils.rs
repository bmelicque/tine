use mylang_core::{pretty_print_error, Module};
use std::{cell::RefCell, rc::Rc};

/// Pretty print all errors found in iterated modules.
///
/// Errors should've been generated during parsing/checking steps.
pub fn print_errors<'a, I>(modules: I) -> bool
where
    I: IntoIterator<Item = &'a Rc<RefCell<Module>>>,
{
    let mut has_errors = false;
    for module in modules {
        for e in &module.borrow().errors {
            has_errors = true;
            pretty_print_error(&e);
        }
    }
    has_errors
}
