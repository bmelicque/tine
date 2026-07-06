pub mod ast;
mod common;
pub mod diagnostics;
pub mod ir;
mod locations;
mod parser;
mod type_checker;
pub mod types;
mod utils;

pub use common::{
    module_path::{ModuleId, ModulePath},
    sources::Source,
    use_decl_to_paths, ModuleImports,
};
pub use diagnostics::*;
pub use locations::{Location, Span};
pub use parser::{parse_project, ParseResult, ProjectParser};
pub use type_checker::*;
pub use utils::pretty_print_error;
