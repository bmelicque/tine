use tine_ast::*;

use crate::expander::Expander;

impl Expander {
    pub fn expand_program(&mut self, mut program: Program) -> Program {
        program.items = program
            .items
            .into_iter()
            .flat_map(|i| self.expand_item(i))
            .collect();
        program
    }

    pub fn expand_item(&mut self, item: Item) -> Vec<Item> {
        use Item::*;
        match item {
            Statement(s) => self
                .expand_statement(s)
                .into_iter()
                .map(|i| Statement(i))
                .collect(),
            i => vec![i],
        }
    }
}
