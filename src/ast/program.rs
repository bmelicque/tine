use super::statements::Statement;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    statements: Vec<Statement>,
}
