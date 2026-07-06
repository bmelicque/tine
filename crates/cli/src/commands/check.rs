use std::path::PathBuf;

use crate::cli::CheckArgs;
use tine_core::{DiagnosticLevel, ModulePath, pretty_print_error};

pub fn run(args: CheckArgs) {
    let module_path = ModulePath::from(&PathBuf::from(args.input));
    let project_result = tine_core::parse_project(module_path.clone());
    let check_result = tine_core::check_project(project_result);
    let error_count = check_result
        .diagnostics
        .iter()
        .flat_map(|(_, diags)| diags)
        .filter(|d| d.level == DiagnosticLevel::Error)
        .count();
    if !check_result.diagnostics.is_empty() {
        for (module_id, diagnostics) in check_result.diagnostics {
            let src = &check_result.sources[&module_id];
            for diag in diagnostics {
                pretty_print_error(src, &diag);
            }
        }
    }

    if error_count > 0 {
        println!("Checked project, found {} error(s)", error_count)
    } else {
        println!("Checked project, found no error!")
    }
}
