mod atoms;
mod expr_to_pattern;
mod structs;
mod tuple;
mod variant;

use crate::{ast, parser2::Parser};

impl Parser<'_> {
    pub fn parse_pattern(&mut self) -> Option<ast::Pattern> {
        unimplemented!()
    }
}
