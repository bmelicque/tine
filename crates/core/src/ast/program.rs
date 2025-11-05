use pest::Span;

use crate::ast::Item;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub span: Span<'static>,
    pub items: Vec<Item>,
}

impl Program {
    pub fn dummy() -> Self {
        Self {
            span: pest::Span::new("", 0, 0).unwrap(),
            items: Vec::new(),
        }
    }
}
