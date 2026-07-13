use tine_common::locations::{Locatable, Location};

use crate::{nodes::ast_struct, Item};

ast_struct!(Program {
    items: Vec<Item>,
});

impl Program {
    pub fn dummy() -> Self {
        Self {
            loc: Location::dummy(),
            items: Vec::new(),
        }
    }
}
