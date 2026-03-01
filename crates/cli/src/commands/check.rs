use crate::{cli::CheckArgs, loader::CliLoader};
use tine_core::{DiagnosticLevel, pretty_print_error};

pub fn run(args: CheckArgs) {
    let session = tine_core::analyze(args.input.into(), Box::new(CliLoader));

    for (&module_id, diagnostics) in session.diagnostics() {
        let src = &session.read_module(module_id).src;
        for diag in diagnostics {
            pretty_print_error(src, diag);
        }
    }
    let error_count = session
        .diagnostics()
        .iter()
        .flat_map(|(_, diags)| diags)
        .filter(|d| d.level == DiagnosticLevel::Error)
        .count();
    if error_count > 0 {
        println!("Checked project, found {} error(s)", error_count)
    } else {
        println!("Checked project, found no error!")
    }
}
