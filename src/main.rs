mod ast;
mod bundler;
mod codegen;
mod parser;
mod type_checker;
mod types;
mod utils;

use std::env;

use bundler::Bundler;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let bundler = Bundler::new();
    let _ = bundler.bundle_entry(&args[1], &args[2]);
}
