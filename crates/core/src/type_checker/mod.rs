mod exhaustiveness;
mod expressions;
mod items;
mod loader;
mod patterns;
mod statements;
mod std;
pub mod substitutions;
pub mod symbols;
mod type_checker;
pub mod type_store;
mod types;
mod utils;

pub use type_checker::{check_project, CheckProjectResult, CheckResult, TypeChecker};
