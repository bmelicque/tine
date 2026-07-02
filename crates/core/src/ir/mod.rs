pub mod expressions;
pub mod statements;
pub mod utils;

pub use expressions::*;
pub use statements::*;
pub use utils::*;

#[derive(Debug, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}
