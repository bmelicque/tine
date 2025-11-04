mod analyzer;
mod ast;
mod bundler;
mod codegen;
mod common;
mod parser;
mod type_checker;
mod types;
mod utils;

use std::{env, path::PathBuf};

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let entry = match to_absolute_path(&args[1].clone()) {
        Ok(path) => path,
        Err(e) => panic!("{:?}", e),
    };

    bundler::transpile(entry, &args[2]);
}

fn to_absolute_path(arg: &str) -> std::io::Result<PathBuf> {
    let path = PathBuf::from(arg);

    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(env::current_dir()?.join(path))
    }
}
