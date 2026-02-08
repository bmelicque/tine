use crate::Location;

use super::{BlockExpression, Expression, Pattern};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl Into<Expression> for Loop {
    fn into(self) -> Expression {
        Expression::Loop(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForExpression {
    pub loc: Location,
    pub condition: Option<Box<Expression>>,
    pub body: Option<BlockExpression>,
}

impl Into<Loop> for ForExpression {
    fn into(self) -> Loop {
        Loop::For(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForInExpression {
    pub loc: Location,
    pub pattern: Option<Box<Pattern>>,
    pub iterable: Option<Box<Expression>>,
    pub body: Option<BlockExpression>,
}

impl Into<Loop> for ForInExpression {
    fn into(self) -> Loop {
        Loop::ForIn(self)
    }
}
