use enum_from_derive::EnumFrom;

use crate::Location;

use super::{BlockExpression, Expression, Pattern};

#[derive(Debug, EnumFrom, Clone, PartialEq, Eq, Hash)]
pub enum Loop {
    For(ForExpression),
    ForIn(ForInExpression),
}

impl Loop {
    pub fn loc(&self) -> Location {
        match self {
            Loop::For(expr) => expr.loc,
            Loop::ForIn(expr) => expr.loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForExpression {
    pub loc: Location,
    pub condition: Option<Box<Expression>>,
    pub body: Option<BlockExpression>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForInExpression {
    pub loc: Location,
    pub pattern: Option<Box<Pattern>>,
    pub iterable: Option<Box<Expression>>,
    pub body: Option<BlockExpression>,
}
