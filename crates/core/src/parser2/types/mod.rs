use crate::{ast, parser2::Parser};

mod atom;
mod binary;
mod function;
mod tuple;
mod unary;

impl Parser<'_> {
    pub fn parse_type(&mut self) -> ast::Type {
        unimplemented!()
    }
}
