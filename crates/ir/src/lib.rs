extern crate self as tine_ir;

pub mod expressions;
mod macros;
pub mod patterns;
pub mod statements;
pub mod utils;
mod walk;

pub use expressions::*;
pub use patterns::*;
pub use statements::*;
pub use utils::*;
pub use walk::*;

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub statements: Vec<Statement>,
}

#[derive(Debug)]
pub enum Node<'a> {
    Pattern(&'a Pattern),
    Expr(&'a Expression),
    Stmt(&'a Statement),
}
impl<'a> From<&'a Pattern> for Node<'a> {
    fn from(value: &'a Pattern) -> Self {
        Self::Pattern(value)
    }
}
impl<'a> From<&'a Expression> for Node<'a> {
    fn from(value: &'a Expression) -> Self {
        Self::Expr(value)
    }
}
impl<'a> From<&'a Statement> for Node<'a> {
    fn from(value: &'a Statement) -> Self {
        Self::Stmt(value)
    }
}

impl<'a> Node<'a> {
    pub fn as_expression(&self) -> Option<&'a Expression> {
        match self {
            Node::Expr(e) => Some(e),
            _ => None,
        }
    }
    pub fn as_statement(&self) -> Option<&'a Statement> {
        match self {
            Node::Stmt(s) => Some(s),
            _ => None,
        }
    }

    pub(crate) fn push_children(&self, stack: &mut Vec<Node<'a>>) {
        match self {
            Node::Pattern(p) => p.push_children(stack),
            Node::Expr(e) => e.push_children(stack),
            Node::Stmt(s) => s.push_children(stack),
        }
    }
}
