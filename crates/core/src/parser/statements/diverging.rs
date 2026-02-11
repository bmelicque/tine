use crate::{
    ast,
    parser::{tokens::Token, Parser},
    Location,
};

impl Parser<'_> {
    pub fn parse_return_statement(&mut self) -> ast::ReturnStatement {
        let kw_range = self.eat(&[Token::Return]);
        let mut loc = self.localize(kw_range);
        let value = self.parse_expression().map(|v| Box::new(v));
        if let Some(value) = &value {
            loc = Location::merge(loc, value.loc());
        }

        ast::ReturnStatement { loc, value }
    }

    pub fn parse_break_statement(&mut self) -> ast::BreakStatement {
        let kw_range = self.eat(&[Token::Break]);
        let mut loc = self.localize(kw_range);
        let value = self.parse_expression().map(|v| Box::new(v));
        if let Some(value) = &value {
            loc = Location::merge(loc, value.loc());
        }

        ast::BreakStatement { loc, value }
    }
}
