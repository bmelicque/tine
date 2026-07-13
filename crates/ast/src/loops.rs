use tine_common::locations::{Locatable, Location};

use crate::nodes::{ast_enum, ast_struct};

use super::{BlockExpression, Expression, Pattern};

ast_enum!(Loop {
    For(ForExpression),
    ForIn(ForInExpression),
});

ast_struct!(ForExpression {
    condition: Option<Box<Expression>>,
    body: Option<BlockExpression>,
});

ast_struct!(ForInExpression {
    pattern: Option<Pattern>,
    iterable: Option<Box<Expression>>,
    body: Option<BlockExpression>,
});
