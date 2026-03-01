use crate::{cli::BuildArgs, loader::CliLoader};
use std::fs;
use tine_transpiler;

pub fn run(args: BuildArgs) {
    let js = tine_transpiler::transpile(&args.input.into(), Box::new(CliLoader))
        .expect("Transpile failed");

    let output = args.output.unwrap_or("out.js".into());

    fs::write(&output, js).expect("Failed to write output");

    println!("Built successfully");
}
