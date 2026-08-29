use tine_ast as ast;

use crate::{tokens::Token, ExpressionCtx, Parser};

mod arrays;
mod atoms;
mod binary;
mod blocks;
mod conditions;
mod elements;
mod functions;
mod loops;
mod matches;
mod path;
mod postfix;
mod structs;
mod tuples;
mod unary;

impl Parser<'_> {
    fn parse_expression(&mut self, ctx: ExpressionCtx) -> Option<ast::Expression> {
        let Some(peeked) = self.tokens.peek().cloned() else {
            return None;
        };
        let Ok(peeked) = peeked.0.clone() else {
            // FIXME: recover
            return Some(ast::Expression::Invalid(ast::InvalidExpression {
                loc: self.localize(peeked.1),
            }));
        };

        match (peeked, ctx.braces()) {
            (Token::Fn, true) => Some(self.parse_function_expression().into()),
            (Token::If, true) => Some(self.parse_condition().into()),
            (Token::LBracket, _) => Some(self.parse_array().into()),
            (Token::LBrace, true) => Some(self.parse_block().into()),
            (Token::Lt, _) => Some(self.parse_element_expression().into()),
            (Token::Match, true) => Some(self.parse_match_expression().into()),
            (Token::For, true) => Some(self.parse_loop_expression().into()),
            _ => self.parse_binary_expression(1, ctx),
        }
    }

    pub fn parse_expression_with_block(&mut self) -> Option<ast::Expression> {
        self.parse_expression(ExpressionCtx::with_braces())
    }

    pub fn parse_expression_without_block(&mut self) -> Option<ast::Expression> {
        self.parse_expression(ExpressionCtx::without_braces())
    }
}
