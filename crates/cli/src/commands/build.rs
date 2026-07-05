use crate::{cli::BuildArgs, loader::CliLoader};
use std::{fs, path::PathBuf};
use tine_core::{ModuleLoader, ModulePath, ProjectParser};
use tine_transpiler;

pub fn run(args: BuildArgs) {
    let module_path = ModulePath::from(&PathBuf::from(args.input));
    let project_result = tine_core::parse_project(module_path.clone());
    let check_result = tine_core::check_project(project_result);

    let js = tine_transpiler::transpile(&args.input.into(), Box::new(CliLoader))
        .expect("Transpile failed");

    let output = args.output.unwrap_or("out.js".into());

    fs::write(&output, js).expect("Failed to write output");

    println!("Built successfully");
}
