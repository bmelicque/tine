use crate::{
    ast,
    parser::{tokens::Token, Parser},
};

mod atom;
mod binary;
mod function;
mod tuple;
mod unary;

impl Parser<'_> {
    pub fn parse_type(&mut self) -> Option<ast::Type> {
        match self.tokens.peek() {
            Some((Ok(Token::Fn), _)) => Some(self.parse_function_type().into()),
            _ => self.parse_binary_type(1),
        }
    }
}
