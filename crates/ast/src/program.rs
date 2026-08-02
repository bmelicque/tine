use tine_common::locations::{Locatable, Location};
use tine_macros::tree_struct;

use crate::Item;

#[tree_struct(untyped)]
pub struct Program {
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
