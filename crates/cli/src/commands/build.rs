use crate::cli::BuildArgs;
use std::{fs, path::PathBuf};
use tine_core::{ModulePath, pretty_print_error};
use tine_transpiler::{self, SwcLoader};

pub fn run(args: BuildArgs) {
    let path_buf = PathBuf::from(args.input).canonicalize().unwrap();
    let module_path = ModulePath::from(&path_buf);
    let project_result = tine_core::parse_project(module_path.clone(), None);
    let check_result = tine_core::check_project(project_result);
    if !check_result.diagnostics.is_empty() {
        for (module_id, diagnostics) in check_result.diagnostics {
            let src = &check_result.sources[&module_id];
            for diag in diagnostics {
                pretty_print_error(src, &diag);
            }
        }
        println!("Found errors, stopped before generating code");
    }

    let loader = SwcLoader {
        sources: check_result.sources,
        ir: check_result.ir,
        ids: check_result.ids,
        types: check_result.types,
        symbols: check_result.symbols,
    };

    let js = tine_transpiler::transpile(&path_buf, loader).expect("Transpile failed");

    let output = args.output.unwrap_or("out.js".into());

    fs::write(&output, js).expect("Failed to write output");

    println!("Built successfully");
}
