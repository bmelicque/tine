use pest::Span;

use super::{BlockExpression, Expression, Pattern};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Loop {
    For(ForExpression),
    ForIn(ForInExpression),
}

impl Loop {
    pub fn as_span(&self) -> Span<'static> {
        match self {
            Loop::For(expr) => expr.span.clone(),
            Loop::ForIn(expr) => expr.span.clone(),
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
    pub span: Span<'static>,
    pub condition: Box<Expression>,
    pub body: BlockExpression,
}

impl Into<Loop> for ForExpression {
    fn into(self) -> Loop {
        Loop::For(self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ForInExpression {
    pub span: Span<'static>,
    pub pattern: Box<Pattern>,
    pub iterable: Box<Expression>,
    pub body: BlockExpression,
}

impl Into<Loop> for ForInExpression {
    fn into(self) -> Loop {
        Loop::ForIn(self)
    }
}
