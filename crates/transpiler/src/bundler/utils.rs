use mylang_core::{pretty_print_error, CheckedModule};

/// Pretty print all errors found in iterated modules.
///
/// Errors should've been generated during parsing/checking steps.
pub fn print_errors<'a, I>(modules: I) -> bool
where
    I: IntoIterator<Item = &'a CheckedModule>,
{
    let mut has_errors = false;
    for module in modules {
        for e in &module.errors {
            has_errors = true;
            pretty_print_error(&module.src, &e);
        }
    }
    has_errors
}
