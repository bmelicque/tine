use crate::{ast::Item, Location};

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub loc: Location,
    pub items: Vec<Item>,
}

impl Program {
    pub fn dummy() -> Self {
        Self {
            loc: Location::dummy(),
            items: Vec::new(),
        }
    }
}
