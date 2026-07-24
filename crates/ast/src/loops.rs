use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::nodes::ast_enum;

use super::{BlockExpression, Expression, Pattern};

ast_enum!(Loop {
    For(ForExpression),
    ForIn(ForInExpression),
});

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ForExpression {
    pub condition: Option<Box<Expression>>,
    pub body: Option<BlockExpression>,
}
impl From<ForExpression> for Expression {
    fn from(node: ForExpression) -> Self {
        Expression::Loop(Loop::For(node))
    }
}

#[tree_struct(untyped)]
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ForInExpression {
    pub pattern: Option<Pattern>,
    pub iterable: Option<Box<Expression>>,
    pub body: Option<BlockExpression>,
}
impl From<ForInExpression> for Expression {
    fn from(node: ForInExpression) -> Self {
        Expression::Loop(Loop::ForIn(node))
    }
}
