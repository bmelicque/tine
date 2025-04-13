mod ast;
mod parser;
mod transpiler;
mod type_checker;
mod types;
mod utils;

use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;

use parser::ParserEngine;
use transpiler::Transpiler;
use type_checker::TypeChecker;

fn main() {
    env::set_var("RUST_BACKTRACE", "1");

    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];
    let input = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(err) => {
            eprintln!("Error reading file {}: {}", filename, err);
            std::process::exit(1);
        }
    };

    let parser = ParserEngine::new();
    let result = parser.parse(&input);
    let Some(ast) = result.node else {
        panic!("parse should make sure that this is Some")
    };
    for error in result.errors {
        utils::pretty_print_error(&input, &error);
    }

    let mut checker = TypeChecker::new();
    match checker.check(&ast) {
        Err(err) => {
            eprintln!("Error type checking file {}: {:?}", filename, err);
            std::process::exit(1);
        }
        _ => (),
    };

    let transpiler = Transpiler::new();
    match transpiler.generate_js(ast) {
        Ok(js_code) => {
            write_output(&args[2], &js_code);
        }
        Err(err) => {
            eprintln!("Code generation error: {}", err);
            std::process::exit(1);
        }
    }
}

fn write_output(path: &str, content: &str) {
    let Ok(mut file) = File::create(path) else {
        eprintln!("Could not create file {}", path);
        std::process::exit(1);
    };
    match file.write_all(content.as_bytes()) {
        Err(err) => {
            eprintln!("Could not write file {}: {}", path, err);
            std::process::exit(1);
        }
        Ok(_) => {}
    }
}
