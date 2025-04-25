mod ast;
mod codegen;
mod parser;
mod type_checker;
mod types;
mod utils;

use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;

use codegen::CodeGenerator;
use parser::ParserEngine;
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
    let static_input: &'static str = Box::leak(input.to_string().into_boxed_str());
    let result = parser.parse(&static_input);
    let Some(ast) = result.node else {
        panic!("parse should make sure that this is Some")
    };
    let has_parse_errors = !result.errors.is_empty();
    for error in result.errors {
        utils::pretty_print_error(&input, &error);
    }

    let mut checker = TypeChecker::new();
    match checker.check(&ast) {
        Err(errors) => {
            for error in errors {
                utils::pretty_print_error(&input, &error);
            }
            std::process::exit(1);
        }
        _ => (),
    };
    if has_parse_errors {
        std::process::exit(1);
    }

    let mut code_generator = CodeGenerator::new();
    match code_generator.generate_js(ast) {
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
