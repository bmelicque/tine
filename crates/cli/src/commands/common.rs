use std::{fs, path::PathBuf};

use tine_common::{module_path::ModulePath, utils::pretty_print_error};
use tine_transpiler::{SwcLoader, TranspileOptions};

pub fn build_once(path_buf: &PathBuf, output: &str, options: TranspileOptions) {
    let module_path = ModulePath::from(path_buf);
    let project_result = tine_parser::parse_project(module_path.clone(), None);
    let check_result = tine_checker::check_project(project_result);

    if !check_result.diagnostics.is_empty() {
        let count = check_result
            .diagnostics
            .iter()
            .fold(0, |sum, (_, list)| sum + list.len());
        for (module_id, diagnostics) in &check_result.diagnostics {
            let src = &check_result.sources[module_id];
            for diag in diagnostics {
                pretty_print_error(src, diag);
            }
        }
        if count > 0 {
            println!("Found errors, stopped before generating code");
            return;
        }
    }

    let loader = SwcLoader {
        sources: check_result.sources,
        ir: check_result.ir,
        ids: check_result.ids,
        types: check_result.types,
        symbols: check_result.symbols,
    };

    let js = match tine_transpiler::transpile(path_buf, loader, options) {
        Ok(js) => js,
        Err(e) => {
            eprintln!("Transpile failed: {e:?}");
            return;
        }
    };

    fs::write(output, js).expect("Failed to write output");
    println!("Built successfully");
}
