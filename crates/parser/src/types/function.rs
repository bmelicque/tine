use tine_ast as ast;
use tine_common::{diagnostics::DiagnosticKind, locations::Location};

use crate::{tokens::Token, Parser};

impl Parser<'_> {
    pub fn parse_function_type(&mut self) -> ast::FunctionType {
        let start_range = self.eat(&[Token::Fn]);
        let start_loc = self.localize(start_range);
        let params = match self.tokens.peek() {
            Some((Ok(Token::LParen), _)) => self.parse_tuple_type(),
            _ => {
                let error_loc = self.next_loc();
                self.error(DiagnosticKind::MissingParams, error_loc);
                return ast::FunctionType {
                    loc: start_loc,
                    params: vec![],
                    returned: None,
                };
            }
        };
        let returned = self.parse_type();
        let loc = match &returned {
            Some(r) => Location::merge(start_loc, r.loc()),
            None => Location::merge(start_loc, params.loc),
        };
        ast::FunctionType {
            loc,
            params: params.elements,
            returned: returned.map(|t| Box::new(t)),
        }
    }
}
