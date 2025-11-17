mod analysis_context;
mod expressions;
mod items;
mod patterns;
mod statements;
mod std;
mod type_checker;
mod type_declaration;
mod types;

pub use analysis_context::{ModuleMetadata, VariableData, VariableRef};
pub use std::dom::dom_metadata;
pub use type_checker::{CheckResult, TypeChecker};
