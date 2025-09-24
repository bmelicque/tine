use std::sync::Arc;

use swc_bundler::{Load, ModuleData};
use swc_common::{FileName, SourceMap};

use crate::{
    bundler::internals::parse_internals, codegen::CodeGenerator, parser::ParserEngine,
    type_checker::TypeChecker, utils,
};

pub struct Loader;

impl Load for Loader {
    fn load(&self, fname: &FileName) -> Result<ModuleData, anyhow::Error> {
        let cm = Arc::new(SourceMap::default());

        if let FileName::Custom(_) = fname {
            return Ok(parse_internals());
        }

        // Read the file contents
        let src = std::fs::read_to_string(match fname {
            FileName::Real(p) => p,
            _ => panic!("Unexpected file name"),
        })?;
        let static_input: &'static str = Box::leak(src.clone().into_boxed_str());

        // Parse the file
        let mut parser = ParserEngine::new();
        let result = parser.parse(&static_input);
        let ast = result.node;
        let has_parse_errors = !result.errors.is_empty();
        for error in result.errors {
            utils::pretty_print_error(static_input, &error);
        }

        let mut checker = TypeChecker::new();
        match checker.check(&ast) {
            Err(errors) => {
                for error in errors {
                    utils::pretty_print_error(static_input, &error);
                }
                std::process::exit(1);
            }
            _ => (),
        };

        if has_parse_errors {
            std::process::exit(1);
        }
        let mut code_generator = CodeGenerator::new(checker.analysis_context);
        let module = code_generator.program_to_swc_module(ast);

        let fm = cm.new_source_file(swc_common::sync::Lrc::new(fname.clone()), src);
        Ok(ModuleData {
            fm,
            module,
            helpers: Default::default(),
        })
    }
}
