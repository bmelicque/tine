use crate::{ast::Item, locations::Span};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub span: Span,
    pub items: Vec<Item>,
}

impl Program {
    pub fn dummy() -> Self {
        Self {
            span: Span::dummy(),
            items: Vec::new(),
        }
    }
}
