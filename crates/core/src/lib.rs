mod analyzer;
pub mod ast;
mod common;
pub mod diagnostics;
pub mod ir;
mod locations;
mod parser;
mod type_checker;
pub mod types;
mod utils;

pub use analyzer::{
    analyze, Module, ModuleId, ModuleLoader, ModulePath, ModuleTypeData, Session, SessionLoader,
    Source,
};
pub use common::{use_decl_to_paths, ModuleImports};
pub use diagnostics::*;
pub use locations::{Location, Span};
pub use type_checker::{
    MemberToken, SymbolData, SymbolKind, SymbolRef, SymbolToken, Token, TypeStore, TypeSymbolBody,
};
pub use utils::pretty_print_error;
