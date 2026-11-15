use crate::{cli::BuildArgs, commands::common::build_once};
use std::path::PathBuf;
use tine_transpiler::{self, TranspileOptions};

pub fn run(args: BuildArgs) {
    let path_buf = PathBuf::from(&args.input).canonicalize().unwrap();
    let output = args.output.clone().unwrap_or("out.js".into());
    let options = TranspileOptions { minify: true };

    build_once(&path_buf, &output, options);
}
