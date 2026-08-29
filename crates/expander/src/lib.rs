mod derives;
mod expander;
mod expressions;
mod items;
mod statements;

use tine_ast::Program;
use tine_common::diagnostics::Diagnostic;

use crate::expander::Expander;

pub fn expand(input: Program) -> (Program, Vec<Diagnostic>) {
    let mut e = Expander::new();
    let output = e.expand_program(input);
    (output, e.diagnostics)
}
