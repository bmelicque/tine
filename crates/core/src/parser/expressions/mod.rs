use crate::{
    ast,
    parser::{tokens::Token, Parser},
};

mod arrays;
mod atoms;
mod binary;
mod blocks;
mod conditions;
mod elements;
mod functions;
mod loops;
mod matches;
mod postfix;
mod tuples;
mod unary;

impl Parser<'_> {
    pub fn parse_expression(&mut self) -> Option<ast::Expression> {
        let Some(peeked) = self.tokens.peek().cloned() else {
            return None;
        };
        let Ok(peeked) = peeked.0.clone() else {
            // FIXME: recover
            return Some(ast::Expression::Invalid(ast::InvalidExpression {
                loc: self.localize(peeked.1),
            }));
        };

        match peeked {
            Token::Fn => Some(self.parse_function_expression().into()),
            Token::If => Some(self.parse_condition().into()),
            Token::LBracket => Some(self.parse_array().into()),
            Token::LBrace => Some(self.parse_block().into()),
            Token::Lt => Some(self.parse_element_expression().into()),
            Token::Match => Some(self.parse_match_expression().into()),
            Token::For => Some(self.parse_loop_expression().into()),
            _ => Some(self.parse_binary_expression(1)),
        }
    }

    pub fn parse_expression_without_block(&mut self) -> Option<ast::Expression> {
        let Some(peeked) = self.tokens.peek().cloned() else {
            return None;
        };
        let Ok(peeked) = peeked.0.clone() else {
            // FIXME: recover
            return Some(ast::Expression::Invalid(ast::InvalidExpression {
                loc: self.localize(peeked.1),
            }));
        };

        match peeked {
            Token::LBracket => Some(self.parse_array().into()),
            _ => Some(self.parse_binary_expression(1)),
        }
    }
}
