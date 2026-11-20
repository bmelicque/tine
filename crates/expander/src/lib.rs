mod derives;
mod expander;
mod expressions;
mod items;
mod statements;

use std::collections::HashMap;

use tine_ast::Program;
use tine_common::diagnostics::Diagnostic;
use tine_parser::ProjectParser;

use crate::expander::Expander;

pub fn expand(input: Program) -> (Program, Vec<Diagnostic>) {
    let mut e = Expander::new();
    let output = e.expand_program(input);
    (output, e.diagnostics)
}

pub fn expand_project(mut project: ProjectParser) -> ProjectParser {
    let (ast, diagnostics): (HashMap<_, _>, HashMap<_, _>) = project
        .ast
        .into_iter()
        .map(|(id, program)| {
            let (expanded, diagnostics) = expand(program);
            ((id, expanded), (id, diagnostics))
        })
        .unzip();

    project.ast = ast;

    for (id, diagnostics) in diagnostics {
        project
            .diagnostics
            .entry(id)
            .or_default()
            .extend(diagnostics);
    }

    project
}
