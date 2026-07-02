mod exhaustiveness;
mod expressions;
mod items;
mod patterns;
mod statements;
pub mod substitutions;
pub mod symbols;
mod test_utils;
mod type_checker;
pub mod type_store;
mod types;
mod utils;

pub use type_checker::{CheckResult, TypeChecker};
